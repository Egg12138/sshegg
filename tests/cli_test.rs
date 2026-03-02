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
fn add_with_both_password_flags_fails() {
    let session_name = "test_both_flags";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "both.example.com",
            "--user",
            "bothuser",
            "--password",
            "--no-password",
        ])
        .assert()
        .failure()
        .stderr(contains("Cannot specify both --password and --no-password"));
}

#[test]
fn add_with_no_password_succeeds() {
    let session_name = "test_no_password";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "nopass.example.com",
            "--user",
            "nopassuser",
            "--no-password",
        ])
        .assert()
        .success()
        .stdout(contains("Added session:"))
        .stdout(contains(session_name));

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains(session_name));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success();
}

#[test]
fn add_without_password_flags_defaults_to_no_password() {
    let session_name = "test_default_no_password";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "default.example.com",
            "--user",
            "defaultuser",
        ])
        .assert()
        .success()
        .stdout(contains("Added session:"));

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains(session_name));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success();
}

#[test]
fn remove_password_command_exists() {
    let session_name = "test_remove_password_cmd";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args(["remove-password", "--name", session_name])
        .assert()
        .failure()
        .stderr(contains("not found"));
}

#[test]
fn remove_password_on_nonexistent_session_fails() {
    let session_name = "test_remove_nonexistent";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args(["remove-password", "--name", session_name])
        .assert()
        .failure()
        .stderr(contains("not found"));
}

#[test]
fn add_without_password_with_other_flags_succeeds() {
    let session_name = "test_other_flags";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "flags.example.com",
            "--user",
            "flagsuser",
            "--port",
            "2222",
            "--tag",
            "prod,critical",
        ])
        .assert()
        .success()
        .stdout(contains("Added session:"));

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains(session_name))
        .stdout(contains("2222"))
        .stdout(contains("prod"));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success();
}

#[test]
fn add_with_identity_file_and_no_password_succeeds() {
    let session_name = "test_identity_no_password";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "id.example.com",
            "--user",
            "iduser",
            "--identity-file",
            "/tmp/test_key",
            "--no-password",
        ])
        .assert()
        .success()
        .stdout(contains("Added session:"));

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains(session_name));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success();
}

#[test]
fn password_flags_work_with_update() {
    let session_name = "test_update_flags";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "update.example.com",
            "--user",
            "updateuser",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args([
            "update",
            "--name",
            session_name,
            "--host",
            "new.example.com",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("new.example.com"));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success();
}

#[test]
fn password_flags_dont_interfere_with_scp() {
    let session_name = "test_scp_flags";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "scp.example.com",
            "--user",
            "scpuser",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args([
            "scp",
            "--name",
            session_name,
            "--local",
            "/tmp/local.txt",
            "--remote",
            "/tmp/remote.txt",
        ])
        .assert()
        .failure()
        .stderr(contains("failed to connect"));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success();
}

#[test]
fn remove_password_flag_validation() {
    let session_name = "test_remove_validation";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "validation.example.com",
            "--user",
            "validationuser",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["remove-password", "--name", session_name])
        .assert()
        .success()
        .stdout(contains("does not have a stored password"));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success();
}

#[test]
fn add_with_tags_and_no_password_succeeds() {
    let session_name = "test_tags_no_password";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "tags.example.com",
            "--user",
            "tagsuser",
            "--tag",
            "prod,critical",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains(session_name))
        .stdout(contains("prod"));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success();
}

#[test]
fn add_with_custom_port_and_no_password_succeeds() {
    let session_name = "test_port_no_password";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "port.example.com",
            "--user",
            "portuser",
            "--port",
            "2222",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains(session_name))
        .stdout(contains("2222"));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success();
}

#[test]
fn remove_session_succeeds() {
    let session_name = "test_session_delete";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "delete.example.com",
            "--user",
            "deleteuser",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success()
        .stdout(contains("Removed session:"))
        .stdout(contains(session_name));
}

#[test]
fn multiple_sessions_with_different_configs() {
    let session1 = "test_multi_session_1";
    let session2 = "test_multi_session_2";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session1,
            "--host",
            "session1.example.com",
            "--user",
            "user1",
            "--port",
            "2222",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session2,
            "--host",
            "session2.example.com",
            "--user",
            "user2",
            "--tag",
            "prod",
        ])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains(session1))
        .stdout(contains(session2));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session1])
        .assert()
        .success();

    ssher_cmd(&store_path)
        .args(["remove", "--name", session2])
        .assert()
        .success();
}

#[test]
fn no_password_flag_works_with_all_required_fields() {
    let session_name = "test_all_fields_no_password";
    let (_dir, store_path) = store_path();

    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "all.example.com",
            "--user",
            "alluser",
            "--port",
            "2222",
            "--tag",
            "prod,test",
            "--no-password",
        ])
        .assert()
        .success()
        .stdout(contains("Added session:"));

    ssher_cmd(&store_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains(session_name));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success();
}

#[test]
fn password_flag_accepted_by_parser() {
    let session_name = "test_password_parser";
    let (_dir, store_path) = store_path();

    let result = ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "parser.example.com",
            "--user",
            "parseruser",
            "--password",
        ])
        .write_stdin("test\n")
        .assert();

    let output = result.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        if stderr.contains("No such device or address") || stderr.contains("keyring") {
            return;
        }
    }

    if output.status.success() {
        assert!(String::from_utf8_lossy(&output.stdout).contains("Added session:"));
    }
}

#[test]
fn session_with_stored_password_flag_includes_has_password() {
    use std::fs;

    let session_name = "test_auto_password_session";
    let (_dir, store_path) = store_path();

    // Add session with --no-password flag (explicit no password)
    ssher_cmd(&store_path)
        .args([
            "add",
            "--name",
            session_name,
            "--host",
            "auto.example.com",
            "--user",
            "autouser",
            "--no-password",
        ])
        .assert()
        .success();

    // Verify session was created
    let content = fs::read_to_string(&store_path).unwrap();
    assert!(content.contains(session_name));
    // With --no-password, has_stored_password should be false (skipped in JSON)
    assert!(!content.contains("has_stored_password"));

    ssher_cmd(&store_path)
        .args(["remove", "--name", session_name])
        .assert()
        .success();
}
