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
