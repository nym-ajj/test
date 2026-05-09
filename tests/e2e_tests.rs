//! E2E binary tests - verify the CLI wrapper works correctly.

use std::{io::Write, process::Command};

use indoc::indoc;

#[test]
fn e2e_binary_smoke() {
    let mut temp = tempfile::NamedTempFile::new().expect("failed to create temp file");
    write!(
        temp,
        "{}",
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            deposit,2,2,50.0
            withdrawal,1,3,25.0
        "}
    )
    .expect("failed to write temp file");

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", temp.path().to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8(output.stdout).expect("invalid utf8");
    assert!(stdout.contains("client,available,held,total,locked"));
    assert!(stdout.contains("75.0000"));
}

#[test]
fn e2e_binary_missing_file() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "nonexistent.csv"])
        .output()
        .expect("failed to run binary");

    assert!(!output.status.success());
}
