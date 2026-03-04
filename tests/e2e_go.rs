use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn store_path() -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let store_path = dir.path().join("sessions.json");
    (dir, store_path)
}

fn ssher_cmd(store_path: &Path) -> assert_cmd::Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("ssher");
    cmd.env("SSHER_STORE", store_path);
    cmd
}

#[cfg(unix)]
#[test]
#[ignore = "requires a reachable SSH test target and SSHER_E2E_GO_IDENTITY_FILE"]
fn go_shell_exit_exits_cleanly_over_pty() {
    let (_dir, store_path) = store_path();
    let session_name = "e2e-go";
    let host = env::var("SSHER_E2E_GO_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let user = env::var("SSHER_E2E_GO_USER")
        .or_else(|_| env::var("USER"))
        .expect("set SSHER_E2E_GO_USER or USER");
    let port = env::var("SSHER_E2E_GO_PORT").unwrap_or_else(|_| "22".to_string());
    let identity_file = env::var("SSHER_E2E_GO_IDENTITY_FILE")
        .expect("set SSHER_E2E_GO_IDENTITY_FILE to a private key that can reach the test host");

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            &host,
            "--user",
            &user,
            "--port",
            &port,
            "--identity-file",
            &identity_file,
        ])
        .assert()
        .success();

    let binary = env::var("CARGO_BIN_EXE_ssher").expect("CARGO_BIN_EXE_ssher");
    let mut child = Command::new("script")
        .env("SSHER_STORE", &store_path)
        .arg("-qefc")
        .arg(format!("{binary} go --name {session_name}"))
        .arg("/dev/null")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn script");

    child
        .stdin
        .as_mut()
        .expect("script stdin")
        .write_all(b"exit\r")
        .expect("write exit");

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            let output = child.wait_with_output().expect("collect output");
            assert!(
                status.success(),
                "go exited unsuccessfully\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }

        if Instant::now() >= deadline {
            child.kill().expect("kill hung child");
            let output = child.wait_with_output().expect("collect hung output");
            panic!(
                "go stayed alive after sending exit\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        sleep(Duration::from_millis(50));
    }
}
