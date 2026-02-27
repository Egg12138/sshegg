use std::process::Command;

fn main() {
    // Get the git commit hash
    let commit_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|hash| hash.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Get the git tag if on a tag
    let git_tag = Command::new("git")
        .args(["describe", "--exact-match", "--tags", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|tag| tag.trim().to_string())
        .unwrap_or_else(|| "none".to_string());

    // Make the commit hash available to the code
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", commit_hash);
    println!("cargo:rustc-env=GIT_TAG={}", git_tag);

    // Rerun if HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
