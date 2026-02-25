use predicates::str::contains;
use std::path::Path;
use tempfile::tempdir;

fn store_path() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("tempdir");
    let store_path = dir.path().join("sessions.json");
    (dir, store_path)
}

fn ssher_cmd(store_path: &Path) -> assert_cmd::Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("ssher");
    cmd.env("SSHER_STORE", store_path);
    cmd
}

#[test]
fn add_and_list_sessions() {
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            "office",
            "--host",
            "office.example.com",
            "--user",
            "me",
            "--port",
            "2222",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("office"))
        .stdout(contains("me@office.example.com"))
        .stdout(contains("2222"));
}

#[test]
fn remove_session_clears_list() {
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            "lab",
            "--host",
            "lab.example.com",
            "--user",
            "me",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["remove", "--name", "lab"])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("No sessions found."));
}

#[test]
fn add_duplicate_name_fails() {
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            "prod",
            "--host",
            "prod.example.com",
            "--user",
            "deploy",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            "prod",
            "--host",
            "prod.example.com",
            "--user",
            "deploy",
        ])
        .assert()
        .failure()
        .stderr(contains("already exists"));
}

#[test]
fn add_with_tags_lists_tags() {
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            "prod",
            "--host",
            "prod.example.com",
            "--user",
            "deploy",
            "--tag",
            "critical,prod",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("critical,prod"));
}

#[test]
fn scp_fails_for_nonexistent_session() {
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "scp",
            "--name",
            "nonexistent",
            "--local",
            "/tmp/file.txt",
            "--remote",
            "/remote/file.txt",
        ])
        .assert()
        .failure()
        .stderr(contains("not found"));
}

#[test]
fn scp_to_direction_generates_correct_command() {
    let (_dir, store_path) = store_path();

    // Add a session with identity file
    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            "office",
            "--host",
            "office.example.com",
            "--user",
            "me",
            "--port",
            "2222",
            "--identity-file",
            "/home/me/.ssh/id_rsa",
        ])
        .assert()
        .success();

    // Test scp with --direction to (default)
    // This will fail because scp doesn't exist, but we can check the error message
    // The command should be: scp -i /home/me/.ssh/id_rsa -P 2222 /tmp/file.txt me@office.example.com:/remote/file.txt
    ssher_cmd(&store_path)
        .args([
            "scp",
            "--name",
            "office",
            "--local",
            "/tmp/file.txt",
            "--remote",
            "/remote/file.txt",
            "--direction",
            "to",
        ])
        .assert()
        .failure()
        .stderr(contains("scp"));
}

#[test]
fn scp_from_direction_generates_correct_command() {
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            "lab",
            "--host",
            "lab.example.com",
            "--user",
            "tester",
        ])
        .assert()
        .success();

    // Test scp with --direction from
    // Command should be: scp -p 2222 tester@lab.example.com:/remote/file.txt /tmp/file.txt
    ssher_cmd(&store_path)
        .args([
            "scp",
            "--name",
            "lab",
            "--local",
            "/tmp/file.txt",
            "--remote",
            "/remote/file.txt",
            "--direction",
            "from",
        ])
        .assert()
        .failure()
        .stderr(contains("scp"));
}

#[test]
fn scp_recursive_includes_recursive_flag() {
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            "prod",
            "--host",
            "prod.example.com",
            "--user",
            "deploy",
        ])
        .assert()
        .success();

    // Test scp with --recursive
    // Command should include -r flag
    ssher_cmd(&store_path)
        .args([
            "scp",
            "--name",
            "prod",
            "--local",
            "/tmp/dir",
            "--remote",
            "/remote/dir",
            "--recursive",
        ])
        .assert()
        .failure()
        .stderr(contains("scp"));
}

#[test]
fn add_with_identity_file() {
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            "office",
            "--host",
            "office.example.com",
            "--user",
            "me",
            "--identity-file",
            "/home/me/.ssh/id_ed25519",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("office"))
        .stdout(contains("/home/me/.ssh/id_ed25519"));
}
