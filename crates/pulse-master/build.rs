use std::process::Command;

fn main() {
    // Get version from git tag (works in CI where we build on a tag)
    let version = Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .trim_start_matches('v')
        .to_string();

    if !version.is_empty() {
        println!("cargo:rustc-env=PULSE_VERSION={version}");
    }
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/tags");
}
