//! E2E binary tests - verify the CLI wrapper works correctly.

use std::{io::Write, process::Command};

use expect_test::{Expect, expect};
use indoc::indoc;

mod common;

fn run_binary(input: &str) -> String {
    let mut temp = tempfile::NamedTempFile::new().expect("failed to create temp file");
    write!(temp, "{}", input).expect("failed to write temp file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            temp.path().to_str().expect("path is valid utf8"),
        ])
        .output()
        .expect("failed to run binary");

    String::from_utf8(output.stdout).expect("invalid utf8")
}

fn check_binary(input: &str, expect: Expect) {
    let stdout = run_binary(input);
    let sorted = common::sorted_output(&stdout);
    expect.assert_eq(&sorted);
}

#[test]
fn e2e_binary_smoke() {
    check_binary(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            deposit,2,2,50.0
            withdrawal,1,3,25.0
        "},
        expect![[r#"
            client,available,held,total,locked
            1,75.0000,0.0000,75.0000,false
            2,50.0000,0.0000,50.0000,false
        "#]],
    );
}

#[test]
fn e2e_binary_whitespace_tolerance() {
    check_binary(
        indoc! {"
            type, client, tx, amount
            deposit, 1, 1, 10.5
            deposit, 2, 2, 20.25
            withdrawal, 1, 3, 5.25
        "},
        expect![[r#"
            client,available,held,total,locked
            1,5.2500,0.0000,5.2500,false
            2,20.2500,0.0000,20.2500,false
        "#]],
    );
}

#[test]
fn e2e_binary_missing_file() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "nonexistent.csv"])
        .output()
        .expect("failed to run binary");

    assert!(!output.status.success());
}
