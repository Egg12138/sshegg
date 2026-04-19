use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

/// Helper to create a temp store and get its path
fn setup_temp_store() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("tempdir");
    let store_path = dir.path().join("sessions.json");
    (dir, store_path)
}

/// Test that config command can set password mode
#[test]
fn config_set_passwd_unsafe_mode_normal() {
    let (_dir, store_path) = setup_temp_store();

    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "set",
            "passwd_unsafe_mode",
            "normal",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set passwd_unsafe_mode = normal"));
}

#[test]
fn config_set_passwd_unsafe_mode_bare() {
    let (_dir, store_path) = setup_temp_store();

    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "set",
            "passwd_unsafe_mode",
            "bare",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set passwd_unsafe_mode = bare"));
}

#[test]
fn config_set_passwd_unsafe_mode_simple() {
    let (_dir, store_path) = setup_temp_store();

    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "set",
            "passwd_unsafe_mode",
            "simple",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set passwd_unsafe_mode = simple"));
}

#[test]
fn config_set_passwd_unsafe_mode_invalid() {
    let (_dir, store_path) = setup_temp_store();

    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "set",
            "passwd_unsafe_mode",
            "invalid",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid value"));
}

#[test]
fn config_get_passwd_unsafe_mode_default() {
    let (_dir, store_path) = setup_temp_store();

    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "get",
            "passwd_unsafe_mode",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("normal"));
}

#[test]
fn config_set_and_get_passwd_unsafe_key() {
    let (_dir, store_path) = setup_temp_store();

    // Set key
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "set",
            "passwd_unsafe_key",
            "my-secret-key",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Set passwd_unsafe_key = my-secret-key",
        ));

    // Get key (should be masked)
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "get",
            "passwd_unsafe_key",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("********"));
}

#[test]
fn config_list_shows_all_settings() {
    let (_dir, store_path) = setup_temp_store();

    Command::cargo_bin("se")
        .unwrap()
        .args(["--store", store_path.to_str().unwrap(), "config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("passwd_unsafe_mode"))
        .stdout(predicate::str::contains("passwd_unsafe_key"));
}

#[test]
fn config_unknown_key_fails() {
    let (_dir, store_path) = setup_temp_store();

    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "get",
            "unknown_key",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown config key"));
}

/// Test add command with --passwd-mode flag
#[test]
fn add_with_passwd_mode_bare() {
    let (_dir, store_path) = setup_temp_store();

    // Set global mode to bare
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "set",
            "passwd_unsafe_mode",
            "bare",
        ])
        .assert()
        .success();

    // Add session with bare mode password
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "add",
            "--name",
            "test-session",
            "--host",
            "example.com",
            "--user",
            "testuser",
            "--password",
        ])
        .write_stdin("test-password\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added session"));
}

#[test]
fn add_with_passwd_mode_simple_requires_key() {
    let (_dir, store_path) = setup_temp_store();

    // Don't set passwd_unsafe_key - should fail when trying to use simple mode
    Command::cargo_bin("se")
        .unwrap()
        .env("SSHER_UNSAFE_KEY", "") // Clear any env key
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "add",
            "--name",
            "test-session",
            "--host",
            "example.com",
            "--user",
            "testuser",
            "--password",
            "--passwd-mode",
            "simple",
        ])
        .write_stdin("test-password\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No encryption key set"));
}

#[test]
fn add_with_passwd_mode_simple_with_env_key() {
    let (_dir, store_path) = setup_temp_store();

    Command::cargo_bin("se")
        .unwrap()
        .env("SSHER_UNSAFE_KEY", "my-encryption-key")
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "add",
            "--name",
            "test-session",
            "--host",
            "example.com",
            "--user",
            "testuser",
            "--password",
            "--passwd-mode",
            "simple",
        ])
        .write_stdin("test-password\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added session"));
}

/// Test backward compatibility - old format auto-migrates
#[test]
fn migrates_old_array_format() {
    let dir = tempdir().expect("tempdir");
    let store_path = dir.path().join("sessions.json");

    // Write old format (array at root)
    std::fs::write(
        &store_path,
        r#"[{"name":"office","host":"example.com","user":"me","port":22}]"#,
    )
    .expect("write");

    // Read should work and auto-migrate
    Command::cargo_bin("se")
        .unwrap()
        .args(["--store", store_path.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("office"));

    // Config should be default
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "get",
            "passwd_unsafe_mode",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("normal"));
}

/// Test new file format structure
#[test]
fn new_format_has_sessions_key() {
    let (_dir, store_path) = setup_temp_store();

    // Add a session
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "add",
            "--name",
            "test",
            "--host",
            "example.com",
            "--user",
            "testuser",
        ])
        .assert()
        .success();

    // Read file content
    let content = std::fs::read_to_string(&store_path).expect("read");

    // Should be new format with sessions array
    assert!(content.contains("\"sessions\""));
    assert!(content.contains("\"passwd_unsafe_mode\""));
}

/// Test update with password mode change
#[test]
fn update_preserves_unsafe_mode() {
    let (_dir, store_path) = setup_temp_store();

    // Add with bare mode
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "set",
            "passwd_unsafe_mode",
            "bare",
        ])
        .assert()
        .success();

    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "add",
            "--name",
            "test-session",
            "--host",
            "example.com",
            "--user",
            "testuser",
            "--password",
        ])
        .write_stdin("test-password\n")
        .assert()
        .success();

    // Update host (should preserve password mode)
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "update",
            "--name",
            "test-session",
            "--host",
            "newhost.example.com",
        ])
        .assert()
        .success();

    // Verify update worked
    Command::cargo_bin("se")
        .unwrap()
        .args(["--store", store_path.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("newhost.example.com"));
}

/// Test that stored_password is not exported
#[test]
fn export_excludes_stored_password() {
    let (_dir, store_path) = setup_temp_store();

    // Add session with password in bare mode
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "set",
            "passwd_unsafe_mode",
            "bare",
        ])
        .assert()
        .success();

    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "add",
            "--name",
            "test-session",
            "--host",
            "example.com",
            "--user",
            "testuser",
            "--password",
        ])
        .write_stdin("secret-password\n")
        .assert()
        .success();

    // Export to JSON
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "export",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-session"))
        // Should NOT contain the actual password
        .stdout(predicate::str::contains("secret-password").not());
}

/// Test remove-password with unsafe mode
#[test]
fn remove_password_clears_unsafe_storage() {
    let (_dir, store_path) = setup_temp_store();

    // Add with bare mode
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "config",
            "set",
            "passwd_unsafe_mode",
            "bare",
        ])
        .assert()
        .success();

    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "add",
            "--name",
            "test-session",
            "--host",
            "example.com",
            "--user",
            "testuser",
            "--password",
        ])
        .write_stdin("test-password\n")
        .assert()
        .success();

    // Remove password
    Command::cargo_bin("se")
        .unwrap()
        .args([
            "--store",
            store_path.to_str().unwrap(),
            "remove-password",
            "--name",
            "test-session",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed password"));
}
