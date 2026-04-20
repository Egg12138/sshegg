//! SSH connection handling using ssh2 crate
//! Supports both key-based and password authentication with auto-detect fallback

use crate::password;
use anyhow::{Context, Result, anyhow};
use ssh2::{KeyboardInteractivePrompt, Prompt, Session as Ssh2Session};
use std::collections::HashSet;
use std::io::IsTerminal;
use std::net::TcpStream;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct AuthConfig {
    pub identity_file: Option<String>,
    pub password: Option<String>,
    pub password_from_keyring: bool,
    pub no_password: bool,
    pub allow_manual_password_prompt: bool,
    pub session_name: Option<String>,
}

pub struct SshConnection {
    session: Ssh2Session,
}

fn poll_shell_fds(
    stdin_fd: i32,
    session_fd: i32,
    watch_stdin: bool,
    watch_write: bool,
) -> Result<bool> {
    let stdin_events = if watch_stdin { libc::POLLIN } else { 0 };
    let session_events = libc::POLLIN | if watch_write { libc::POLLOUT } else { 0 };
    let mut fds = [
        libc::pollfd {
            fd: stdin_fd,
            events: stdin_events,
            revents: 0,
        },
        libc::pollfd {
            fd: session_fd,
            events: session_events,
            revents: 0,
        },
    ];

    loop {
        let rc = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 100) };
        if rc >= 0 {
            return Ok(watch_stdin && (fds[0].revents & libc::POLLIN) != 0);
        }

        let err = std::io::Error::last_os_error();
        if err.kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        return Err(err).context("failed to poll SSH shell file descriptors");
    }
}

fn current_pty_size() -> Option<(u32, u32)> {
    crossterm::terminal::size()
        .ok()
        .map(|(cols, rows)| (u32::from(cols), u32::from(rows)))
}

fn sync_pty_size_if_needed<F>(
    last_size: &mut Option<(u32, u32)>,
    current_size: Option<(u32, u32)>,
    mut request_resize: F,
) -> Result<bool>
where
    F: FnMut(u32, u32) -> Result<()>,
{
    let Some((cols, rows)) = current_size else {
        return Ok(false);
    };

    if *last_size == Some((cols, rows)) {
        return Ok(false);
    }

    request_resize(cols, rows)?;
    *last_size = Some((cols, rows));
    Ok(true)
}

#[derive(Clone, Copy, Debug)]
struct AuthMethodHints {
    publickey: bool,
    password: bool,
    keyboard_interactive: bool,
}

impl AuthMethodHints {
    fn permissive() -> Self {
        Self {
            publickey: true,
            password: true,
            keyboard_interactive: true,
        }
    }

    fn from_server(methods: &str) -> Self {
        let mut parsed = Self {
            publickey: false,
            password: false,
            keyboard_interactive: false,
        };

        for method in methods.split(',').map(str::trim).filter(|m| !m.is_empty()) {
            match method {
                "publickey" => parsed.publickey = true,
                "password" => parsed.password = true,
                "keyboard-interactive" => parsed.keyboard_interactive = true,
                _ => {}
            }
        }

        if parsed.publickey || parsed.password || parsed.keyboard_interactive {
            parsed
        } else {
            Self::permissive()
        }
    }
}

struct StaticPasswordPrompter {
    password: String,
}

impl StaticPasswordPrompter {
    fn new(password: &str) -> Self {
        Self {
            password: password.to_string(),
        }
    }
}

impl KeyboardInteractivePrompt for StaticPasswordPrompter {
    fn prompt<'a>(
        &mut self,
        _username: &str,
        _instructions: &str,
        prompts: &[Prompt<'a>],
    ) -> Vec<String> {
        prompts.iter().map(|_| self.password.clone()).collect()
    }
}

fn is_likely_invalid_password_error(err: &anyhow::Error) -> bool {
    let message = format!("{err:#}").to_lowercase();
    message.contains("authentication failed")
        || message.contains("invalid password")
        || message.contains("incorrect password")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn expand_tilde_path(path: &str, home: Option<&Path>) -> PathBuf {
    if path == "~" {
        return home
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(path));
    }

    if let Some(suffix) = path.strip_prefix("~/")
        && let Some(home) = home
    {
        return home.join(suffix);
    }

    PathBuf::from(path)
}

fn push_unique_path(paths: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, candidate: PathBuf) {
    if seen.insert(candidate.clone()) {
        paths.push(candidate);
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn parse_remote_completion_input(input: &str) -> (String, String) {
    if input.is_empty() {
        return (".".to_string(), String::new());
    }

    if input == "/" {
        return ("/".to_string(), String::new());
    }

    if let Some(directory) = input.strip_suffix('/') {
        if directory.is_empty() {
            return ("/".to_string(), String::new());
        }
        return (directory.to_string(), String::new());
    }

    if let Some((directory, prefix)) = input.rsplit_once('/') {
        if directory.is_empty() {
            return ("/".to_string(), prefix.to_string());
        }
        return (directory.to_string(), prefix.to_string());
    }

    (".".to_string(), input.to_string())
}

fn build_remote_suggestions(
    directory: &str,
    prefix: &str,
    entries: &[(String, bool)],
) -> Vec<String> {
    let mut suggestions = entries
        .iter()
        .filter(|(name, _)| name.starts_with(prefix))
        .map(|(name, is_dir)| {
            let mut path = match directory {
                "." => name.clone(),
                "/" => format!("/{name}"),
                _ => format!("{directory}/{name}"),
            };
            if *is_dir {
                path.push('/');
            }
            path
        })
        .collect::<Vec<_>>();
    suggestions.sort();
    suggestions
}

fn discover_identity_files_in_dir(ssh_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    let preferred = [
        "id_ed25519",
        "id_ecdsa",
        "id_ecdsa_sk",
        "id_ed25519_sk",
        "id_rsa",
        "id_dsa",
        "id_xmss",
    ];

    for name in preferred {
        let candidate = ssh_dir.join(name);
        if candidate.is_file() {
            push_unique_path(&mut candidates, &mut seen, candidate);
        }
    }

    if let Ok(entries) = std::fs::read_dir(ssh_dir) {
        let mut extra = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if file_name.starts_with("id_") && !file_name.ends_with(".pub") {
                extra.push(path);
            }
        }

        extra.sort();
        for candidate in extra {
            push_unique_path(&mut candidates, &mut seen, candidate);
        }
    }

    candidates
}

fn collect_identity_candidates(session_identity: Option<&str>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let home = home_dir();

    if let Some(session_identity) = session_identity {
        let identity = expand_tilde_path(session_identity, home.as_deref());
        push_unique_path(&mut candidates, &mut seen, identity);
        return candidates;
    }

    if let Some(home) = home.as_deref() {
        let ssh_dir = home.join(".ssh");
        for candidate in discover_identity_files_in_dir(&ssh_dir) {
            push_unique_path(&mut candidates, &mut seen, candidate);
        }
    }

    candidates
}

impl SshConnection {
    pub fn connect(host: &str, port: u16, user: &str, auth_config: &AuthConfig) -> Result<Self> {
        let tcp = TcpStream::connect(format!("{}:{}", host, port))
            .context("failed to connect to SSH server")?;
        let mut sess = Ssh2Session::new().context("failed to create SSH session")?;
        sess.set_tcp_stream(tcp);
        sess.handshake().context("SSH handshake failed")?;

        // Build the keyring lookup key from session_name or fallback to user@host
        let keyring_key: String = match &auth_config.session_name {
            Some(name) => name.clone(),
            None => format!("{}@{}", user, host),
        };

        let auth_result = Self::authenticate(&sess, host, user, auth_config, &keyring_key);

        auth_result.context("SSH authentication failed")?;

        Ok(Self { session: sess })
    }

    fn authenticate(
        sess: &Ssh2Session,
        host: &str,
        user: &str,
        auth_config: &AuthConfig,
        keyring_key: &str,
    ) -> Result<()> {
        let auth_methods = match sess.auth_methods(user) {
            Ok(methods) => AuthMethodHints::from_server(methods),
            Err(_) => AuthMethodHints::permissive(),
        };
        let mut attempts = Vec::new();
        let mut password_attempted = false;
        let mut password_rejected = false;

        if auth_methods.publickey {
            let has_explicit_identity = auth_config.identity_file.is_some();
            if !has_explicit_identity {
                match Self::try_agent_auth(sess, user) {
                    Ok(()) => return Ok(()),
                    Err(err) => attempts.push(format!("SSH agent auth failed: {err:#}")),
                }
            } else {
                attempts.push(
                    "session identity configured; skipping SSH agent auth attempts".to_string(),
                );
            }

            let identity_candidates =
                collect_identity_candidates(auth_config.identity_file.as_deref());
            if identity_candidates.is_empty() {
                attempts.push("no local identity files discovered in ~/.ssh".to_string());
            }

            for identity in identity_candidates {
                match Self::try_key_auth(sess, user, &identity) {
                    Ok(()) => return Ok(()),
                    Err(err) => attempts.push(format!(
                        "key auth failed for '{}': {err:#}",
                        identity.display()
                    )),
                }
            }
        } else {
            attempts.push("server does not advertise publickey authentication".to_string());
        }

        if !auth_config.no_password {
            if let Some(password) = auth_config.password.as_deref() {
                match Self::try_password_auth(sess, user, password, &auth_methods) {
                    Ok(()) => return Ok(()),
                    Err(err) => {
                        password_attempted = true;
                        if is_likely_invalid_password_error(&err) {
                            password_rejected = true;
                        }
                        attempts.push(format!("explicit password auth failed: {err:#}"));
                    }
                }
            }

            if auth_config.password_from_keyring {
                match password::get_password(keyring_key) {
                    Ok(Some(password)) => {
                        match Self::try_password_auth(sess, user, &password, &auth_methods) {
                            Ok(()) => return Ok(()),
                            Err(err) => {
                                password_attempted = true;
                                if is_likely_invalid_password_error(&err) {
                                    password_rejected = true;
                                }
                                attempts.push(format!("keyring password auth failed: {err:#}"))
                            }
                        }
                    }
                    Ok(None) => {
                        attempts.push("no keyring password found for this session".to_string())
                    }
                    Err(err) => attempts.push(format!("failed to read keyring password: {err:#}")),
                }
            }

            if auth_config.allow_manual_password_prompt {
                match Self::prompt_for_password(host, user)? {
                    Some(password) => {
                        match Self::try_password_auth(sess, user, &password, &auth_methods) {
                            Ok(()) => return Ok(()),
                            Err(err) => {
                                password_attempted = true;
                                if is_likely_invalid_password_error(&err) {
                                    password_rejected = true;
                                }
                                attempts.push(format!("manual password auth failed: {err:#}"))
                            }
                        }
                    }
                    None => attempts.push(
                        "manual password prompt skipped (non-interactive or empty)".to_string(),
                    ),
                }
            }
        } else {
            attempts.push("password authentication disabled by configuration".to_string());
        }

        let summary = if attempts.is_empty() {
            "no auth attempts were performed".to_string()
        } else {
            attempts.join(" | ")
        };

        if password_rejected {
            return Err(anyhow!(
                "Password authentication was rejected (likely incorrect password). {summary}"
            ));
        }

        if password_attempted {
            return Err(anyhow!("Password authentication failed. {summary}"));
        }

        Err(anyhow!("No authentication method succeeded: {summary}"))
    }

    fn try_agent_auth(sess: &Ssh2Session, user: &str) -> Result<()> {
        let mut agent = sess.agent().context("failed to access SSH agent")?;
        agent.connect().context("failed to connect to SSH agent")?;
        agent
            .list_identities()
            .context("failed to list SSH agent identities")?;
        let identities = agent
            .identities()
            .context("failed to read SSH agent identities")?;

        if identities.is_empty() {
            let _ = agent.disconnect();
            return Err(anyhow!("no identities available in SSH agent"));
        }

        let mut failures = Vec::new();
        for identity in identities {
            let label = match identity.comment().trim() {
                "" => "<unnamed identity>".to_string(),
                comment => comment.to_string(),
            };
            match agent.userauth(user, &identity) {
                Ok(()) => {
                    let _ = agent.disconnect();
                    return Ok(());
                }
                Err(err) => failures.push(format!("{label}: {err}")),
            }
        }

        let _ = agent.disconnect();
        Err(anyhow!(
            "all SSH agent identities failed ({})",
            failures.join("; ")
        ))
    }

    fn try_key_auth(sess: &Ssh2Session, user: &str, identity_path: &Path) -> Result<()> {
        if !identity_path.is_file() {
            return Err(anyhow!(
                "identity file does not exist: {}",
                identity_path.display()
            ));
        }

        sess.userauth_pubkey_file(user, None, identity_path, None)
            .context("key authentication failed")?;

        Ok(())
    }

    fn try_password_auth(
        sess: &Ssh2Session,
        user: &str,
        password: &str,
        auth_methods: &AuthMethodHints,
    ) -> Result<()> {
        let mut errors = Vec::new();

        if auth_methods.password {
            match sess.userauth_password(user, password) {
                Ok(()) => return Ok(()),
                Err(err) => errors.push(format!("password method failed: {err}")),
            }
        }

        if auth_methods.keyboard_interactive {
            let mut prompter = StaticPasswordPrompter::new(password);
            match sess.userauth_keyboard_interactive(user, &mut prompter) {
                Ok(()) => return Ok(()),
                Err(err) => errors.push(format!("keyboard-interactive method failed: {err}")),
            }
        }

        if errors.is_empty() {
            return Err(anyhow!(
                "server does not advertise password or keyboard-interactive authentication"
            ));
        }

        Err(anyhow!(errors.join("; ")))
    }

    fn prompt_for_password(host: &str, user: &str) -> Result<Option<String>> {
        if !std::io::stdin().is_terminal() {
            return Ok(None);
        }

        let prompt = format!("Password for {}@{}: ", user, host);
        let password =
            rpassword::prompt_password(prompt).context("failed to read password from terminal")?;
        if password.is_empty() {
            Ok(None)
        } else {
            Ok(Some(password))
        }
    }

    #[allow(dead_code)]
    pub fn exec(&mut self, command: &str) -> Result<String> {
        let mut channel = self
            .session
            .channel_session()
            .context("failed to open SSH channel")?;
        channel.exec(command).context("failed to execute command")?;

        use std::io::Read;
        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .context("failed to read command output")?;

        channel
            .wait_close()
            .context("failed to wait for channel close")?;

        Ok(output)
    }

    pub fn list_remote_path_suggestions(&mut self, input: &str) -> Result<Vec<String>> {
        let (directory, prefix) = parse_remote_completion_input(input);
        let script = format!(
            "LC_ALL=C find {} -maxdepth 1 -mindepth 1 -printf '%f\\t%y\\n' 2>/dev/null",
            shell_quote(&directory)
        );
        let command = format!("sh -lc {}", shell_quote(&script));
        let output = self.exec(&command)?;

        let entries = output
            .lines()
            .filter_map(|line| {
                let (name, kind) = line.split_once('\t')?;
                Some((name.to_string(), kind == "d"))
            })
            .collect::<Vec<_>>();

        Ok(build_remote_suggestions(&directory, &prefix, &entries))
    }

    pub fn shell(&mut self) -> Result<()> {
        use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
        use std::io::{Read, Write};

        let mut channel = self
            .session
            .channel_session()
            .context("failed to open SSH channel")?;

        let mut synced_pty_size = current_pty_size();
        let pty_dims = synced_pty_size.map(|(cols, rows)| (cols, rows, 0, 0));

        channel
            .request_pty("xterm-256color", None, pty_dims)
            .context("failed to request PTY")?;

        channel.shell().context("failed to start shell")?;

        // Save terminal state and set up raw mode
        disable_raw_mode().ok(); // Ensure we start clean
        enable_raw_mode()?;

        let mut stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        let mut channel_stdin = channel.stream(0);
        let mut channel_stdout = channel.stream(0);
        let mut channel_stderr = channel.stderr();
        let stdin_fd = stdin.as_raw_fd();
        let session_fd = self.session.as_raw_fd();
        let mut stdin_closed = false;
        let mut sent_eof = false;
        let mut pending_input = Vec::new();
        let mut stdin_buffer = [0u8; 8192];
        let mut stdout_buffer = [0u8; 8192];
        let mut stderr_buffer = [0u8; 8192];

        self.session.set_blocking(false);
        let shell_result = (|| -> Result<()> {
            loop {
                let mut progressed = false;

                progressed |= sync_pty_size_if_needed(
                    &mut synced_pty_size,
                    current_pty_size(),
                    |cols, rows| {
                        channel
                            .request_pty_size(cols, rows, None, None)
                            .context("failed to update SSH PTY size")
                    },
                )?;

                loop {
                    match channel_stdout.read(&mut stdout_buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            stdout
                                .write_all(&stdout_buffer[..n])
                                .context("failed to write SSH stdout")?;
                            progressed = true;
                        }
                        Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(err) => return Err(err).context("failed to read SSH stdout"),
                    }
                }

                loop {
                    match channel_stderr.read(&mut stderr_buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            stdout
                                .write_all(&stderr_buffer[..n])
                                .context("failed to write SSH stderr")?;
                            progressed = true;
                        }
                        Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(err) => return Err(err).context("failed to read SSH stderr"),
                    }
                }

                if !pending_input.is_empty() {
                    match channel_stdin.write(&pending_input) {
                        Ok(0) => {}
                        Ok(n) => {
                            pending_input.drain(..n);
                            progressed = true;
                        }
                        Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(err) => return Err(err).context("failed to write SSH stdin"),
                    }
                } else if stdin_closed && !sent_eof {
                    match channel.send_eof() {
                        Ok(()) => {
                            sent_eof = true;
                            progressed = true;
                        }
                        // libssh2 reports EAGAIN as session error -37 when nonblocking I/O
                        // needs the socket to become writable before sending EOF.
                        Err(ref err) if matches!(err.code(), ssh2::ErrorCode::Session(code) if code == -37) =>
                            {}
                        Err(err) => return Err(err).context("failed to send SSH EOF"),
                    }
                }

                stdout.flush().context("failed to flush terminal output")?;

                if channel.eof() {
                    break;
                }

                let stdin_ready = poll_shell_fds(
                    stdin_fd,
                    session_fd,
                    !stdin_closed,
                    !pending_input.is_empty() || (stdin_closed && !sent_eof),
                )?;

                if stdin_ready {
                    match stdin.read(&mut stdin_buffer) {
                        Ok(0) => stdin_closed = true,
                        Ok(n) => pending_input.extend_from_slice(&stdin_buffer[..n]),
                        Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(err) => return Err(err).context("failed to read terminal input"),
                    }
                } else if !stdin_closed {
                    std::thread::sleep(Duration::from_millis(5));
                }

                if !progressed && stdin_closed && sent_eof {
                    std::thread::sleep(Duration::from_millis(5));
                }
            }

            Ok(())
        })();

        self.session.set_blocking(true);
        disable_raw_mode()?;

        shell_result
    }

    pub fn upload(&self, local_path: &std::path::Path, remote_path: &str) -> Result<()> {
        let remote_path = std::path::Path::new(remote_path);
        let mut channel = self
            .session
            .scp_send(remote_path, 0o644, 0, None)
            .context("failed to start SCP send")?;

        let mut file = std::fs::File::open(local_path).context("failed to open local file")?;

        use std::io::{Read, Write};
        let mut buffer = [0u8; 8192];
        loop {
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    channel
                        .write_all(&buffer[..n])
                        .context("failed to write to remote file")?;
                }
                Err(e) => return Err(e).context("failed to read local file"),
            }
        }

        channel.send_eof().context("failed to send EOF")?;
        channel
            .wait_close()
            .context("failed to wait for channel close")?;

        Ok(())
    }

    pub fn download(&self, remote_path: &str, local_path: &std::path::Path) -> Result<()> {
        let remote_path = std::path::Path::new(remote_path);
        let (mut channel, _stat) = self
            .session
            .scp_recv(remote_path)
            .context("failed to start SCP receive")?;

        let mut file = std::fs::File::create(local_path).context("failed to create local file")?;

        use std::io::{Read, Write};
        let mut buffer = [0u8; 8192];
        loop {
            match channel.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    file.write_all(&buffer[..n])
                        .context("failed to write to local file")?;
                }
                Err(e) => return Err(e).context("failed to read remote file"),
            }
        }

        channel
            .wait_close()
            .context("failed to wait for channel close")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn auth_method_hints_parse_known_methods() {
        let hints = AuthMethodHints::from_server("publickey,password");
        assert!(hints.publickey);
        assert!(hints.password);
        assert!(!hints.keyboard_interactive);
    }

    #[test]
    fn auth_method_hints_empty_input_is_permissive() {
        let hints = AuthMethodHints::from_server("");
        assert!(hints.publickey);
        assert!(hints.password);
        assert!(hints.keyboard_interactive);
    }

    #[test]
    fn expand_tilde_path_uses_home_when_available() {
        let home = Path::new("/home/tester");
        assert_eq!(
            expand_tilde_path("~/id_ed25519", Some(home)),
            PathBuf::from("/home/tester/id_ed25519")
        );
        assert_eq!(
            expand_tilde_path("~", Some(home)),
            PathBuf::from("/home/tester")
        );
    }

    #[test]
    fn discover_identity_files_ignores_public_keys_and_prefers_standard_order() {
        let dir = tempdir().unwrap();
        let ssh_dir = dir.path().join(".ssh");
        std::fs::create_dir_all(&ssh_dir).unwrap();

        std::fs::write(ssh_dir.join("id_custom"), "").unwrap();
        std::fs::write(ssh_dir.join("id_rsa"), "").unwrap();
        std::fs::write(ssh_dir.join("id_ed25519.pub"), "").unwrap();
        std::fs::write(ssh_dir.join("config"), "").unwrap();
        std::fs::write(ssh_dir.join("id_ecdsa"), "").unwrap();

        let identities = discover_identity_files_in_dir(&ssh_dir);

        assert_eq!(identities[0], ssh_dir.join("id_ecdsa"));
        assert_eq!(identities[1], ssh_dir.join("id_rsa"));
        assert_eq!(identities[2], ssh_dir.join("id_custom"));
        assert_eq!(identities.len(), 3);
    }

    #[test]
    fn collect_identity_candidates_with_explicit_identity_only_returns_explicit_path() {
        let explicit = "/tmp/test-key";
        let identities = collect_identity_candidates(Some(explicit));
        assert_eq!(identities, vec![PathBuf::from(explicit)]);
    }

    #[test]
    fn invalid_password_error_detection_matches_authentication_failed() {
        let err = anyhow!("password method failed: [-18] Authentication failed");
        assert!(is_likely_invalid_password_error(&err));
    }

    #[test]
    fn invalid_password_error_detection_ignores_unrelated_errors() {
        let err = anyhow!("server does not advertise password authentication");
        assert!(!is_likely_invalid_password_error(&err));
    }

    #[test]
    fn remote_path_query_splits_directory_and_prefix() {
        assert_eq!(
            parse_remote_completion_input("/var/lo"),
            ("/var".to_string(), "lo".to_string())
        );
        assert_eq!(
            parse_remote_completion_input("/var/log/"),
            ("/var/log".to_string(), "".to_string())
        );
        assert_eq!(
            parse_remote_completion_input("notes"),
            (".".to_string(), "notes".to_string())
        );
    }

    #[test]
    fn remote_completion_candidates_filter_and_mark_directories() {
        let suggestions = build_remote_suggestions(
            "/var",
            "lo",
            &[
                ("log".to_string(), true),
                ("local".to_string(), true),
                ("tmp".to_string(), true),
            ],
        );

        assert_eq!(
            suggestions,
            vec!["/var/local/".to_string(), "/var/log/".to_string()]
        );
    }

    #[test]
    fn sync_pty_size_if_needed_requests_resize_when_terminal_size_changes() {
        let mut last_size = Some((80, 24));
        let mut requested = Vec::new();

        let changed = sync_pty_size_if_needed(&mut last_size, Some((120, 40)), |cols, rows| {
            requested.push((cols, rows));
            Ok(())
        })
        .expect("resize sync should succeed");

        assert!(changed);
        assert_eq!(requested, vec![(120, 40)]);
        assert_eq!(last_size, Some((120, 40)));
    }

    #[test]
    fn sync_pty_size_if_needed_skips_duplicate_terminal_sizes() {
        let mut last_size = Some((120, 40));
        let mut requested = Vec::new();

        let changed = sync_pty_size_if_needed(&mut last_size, Some((120, 40)), |cols, rows| {
            requested.push((cols, rows));
            Ok(())
        })
        .expect("duplicate resize check should succeed");

        assert!(!changed);
        assert!(requested.is_empty());
        assert_eq!(last_size, Some((120, 40)));
    }
}
