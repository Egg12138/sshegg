//! SSH connection handling using ssh2 crate
//! Supports both key-based and password authentication with auto-detect fallback

use crate::password;
use anyhow::{Context, Result, anyhow};
use ssh2::Session as Ssh2Session;
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct AuthConfig {
    pub identity_file: Option<String>,
    pub password: Option<String>,
    pub password_from_keyring: bool,
    pub no_password: bool,
}

pub struct SshConnection {
    session: Ssh2Session,
}

impl SshConnection {
    pub fn connect(host: &str, port: u16, user: &str, auth_config: &AuthConfig) -> Result<Self> {
        let tcp = TcpStream::connect(format!("{}:{}", host, port))
            .context("failed to connect to SSH server")?;
        let mut sess = Ssh2Session::new().context("failed to create SSH session")?;
        sess.set_tcp_stream(tcp);
        sess.handshake().context("SSH handshake failed")?;

        let auth_result = if auth_config.no_password {
            if let Some(identity) = &auth_config.identity_file {
                Self::try_key_auth(&sess, user, identity)
            } else {
                Err(anyhow!("No authentication method available"))
            }
        } else if let Some(identity) = &auth_config.identity_file {
            match Self::try_key_auth(&sess, user, identity) {
                Ok(_) => Ok(()),
                Err(_) => {
                    if let Some(pwd) = &auth_config.password {
                        Self::try_password_auth(&sess, user, pwd)
                    } else if auth_config.password_from_keyring {
                        if let Some(pwd) = password::get_password(&format!("{}@{}", user, host))? {
                            Self::try_password_auth(&sess, user, &pwd)
                        } else {
                            Err(anyhow!("Authentication failed"))
                        }
                    } else {
                        Err(anyhow!("Authentication failed"))
                    }
                }
            }
        } else if let Some(pwd) = &auth_config.password {
            Self::try_password_auth(&sess, user, pwd)
        } else if auth_config.password_from_keyring {
            if let Some(pwd) = password::get_password(&format!("{}@{}", user, host))? {
                Self::try_password_auth(&sess, user, &pwd)
            } else {
                Err(anyhow!("No password available"))
            }
        } else {
            Err(anyhow!("No authentication method available"))
        };

        auth_result.context("SSH authentication failed")?;

        Ok(Self { session: sess })
    }

    fn try_key_auth(sess: &Ssh2Session, user: &str, identity_path: &str) -> Result<()> {
        let path = Path::new(identity_path);
        if !path.exists() {
            return Err(anyhow!("Identity file does not exist: {}", identity_path));
        }

        sess.userauth_pubkey_file(user, None, path, None)
            .context("key authentication failed")?;

        Ok(())
    }

    fn try_password_auth(sess: &Ssh2Session, user: &str, password: &str) -> Result<()> {
        sess.userauth_password(user, password)
            .context("password authentication failed")?;

        Ok(())
    }

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

    pub fn shell(&mut self) -> Result<()> {
        let mut channel = self
            .session
            .channel_session()
            .context("failed to open SSH channel")?;

        channel
            .request_pty("xterm-termite", None, None)
            .context("failed to request PTY")?;

        channel.shell().context("failed to start shell")?;

        let mut stdin = std::io::stdin();
        let stdout = Arc::new(Mutex::new(std::io::stdout()));

        use std::io::{Read, Write};

        let mut channel_stdin = channel.stream(0);
        let mut channel_stdout = channel.stream(0);
        let mut channel_stderr = channel.stderr();

        let stdin_handle = std::thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match stdin.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if channel_stdin.write_all(&buffer[..n]).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let stdout_handle = {
            let stdout = stdout.clone();
            std::thread::spawn(move || {
                let mut buffer = [0u8; 8192];
                loop {
                    match channel_stdout.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Ok(mut out) = stdout.lock()
                                && out.write_all(&buffer[..n]).is_err()
                            {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            })
        };

        let stderr_handle = {
            let stdout = stdout.clone();
            std::thread::spawn(move || {
                let mut buffer = [0u8; 8192];
                loop {
                    match channel_stderr.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Ok(mut out) = stdout.lock()
                                && out.write_all(&buffer[..n]).is_err()
                            {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            })
        };

        stdin_handle.join().ok();
        stdout_handle.join().ok();
        stderr_handle.join().ok();

        Ok(())
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
    use crate::model::Session;

    #[test]
    fn auth_config_from_session_with_stored_password() {
        // Verify that a session with has_stored_password=true triggers keyring lookup
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "testuser".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: true,
        };

        // Simulate what run_ssh() does
        let auth_config = AuthConfig {
            identity_file: session
                .identity_file
                .as_ref()
                .map(|p| p.display().to_string()),
            password_from_keyring: session.has_stored_password,
            password: None,
            no_password: !session.has_stored_password && session.identity_file.is_none(),
        };

        assert!(
            auth_config.password_from_keyring,
            "Should attempt keyring lookup"
        );
        assert!(!auth_config.no_password, "Should not be no_password mode");
        assert!(auth_config.identity_file.is_none());
    }

    #[test]
    fn auth_config_from_session_without_stored_password() {
        // Verify that a session without stored password does NOT trigger keyring lookup
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "testuser".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: false,
        };

        let auth_config = AuthConfig {
            identity_file: session
                .identity_file
                .as_ref()
                .map(|p| p.display().to_string()),
            password_from_keyring: session.has_stored_password,
            password: None,
            no_password: !session.has_stored_password && session.identity_file.is_none(),
        };

        assert!(
            !auth_config.password_from_keyring,
            "Should NOT attempt keyring lookup"
        );
        assert!(auth_config.no_password, "Should be no_password mode");
    }

    #[test]
    fn auth_config_from_session_with_identity_file() {
        // Verify that a session with identity file sets no_password=false
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "testuser".to_string(),
            port: 22,
            identity_file: Some(std::path::PathBuf::from("/home/user/.ssh/id_rsa")),
            tags: vec![],
            last_connected_at: None,
            has_stored_password: false,
        };

        let auth_config = AuthConfig {
            identity_file: session
                .identity_file
                .as_ref()
                .map(|p| p.display().to_string()),
            password_from_keyring: session.has_stored_password,
            password: None,
            no_password: !session.has_stored_password && session.identity_file.is_none(),
        };

        assert!(
            !auth_config.password_from_keyring,
            "Should NOT attempt keyring lookup"
        );
        assert!(
            !auth_config.no_password,
            "Should NOT be no_password mode (has identity file)"
        );
        assert!(auth_config.identity_file.is_some());
    }

    #[test]
    fn auth_config_from_session_with_both_auth_methods() {
        // Verify that a session with both identity file and stored password
        // will attempt keyring lookup as fallback
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "testuser".to_string(),
            port: 22,
            identity_file: Some(std::path::PathBuf::from("/home/user/.ssh/id_rsa")),
            tags: vec![],
            last_connected_at: None,
            has_stored_password: true,
        };

        let auth_config = AuthConfig {
            identity_file: session
                .identity_file
                .as_ref()
                .map(|p| p.display().to_string()),
            password_from_keyring: session.has_stored_password,
            password: None,
            no_password: !session.has_stored_password && session.identity_file.is_none(),
        };

        assert!(
            auth_config.password_from_keyring,
            "Should attempt keyring lookup as fallback"
        );
        assert!(!auth_config.no_password, "Should NOT be no_password mode");
        assert!(auth_config.identity_file.is_some());
    }
}
