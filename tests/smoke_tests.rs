//! Smoke tests - basic happy path verification.

use expect_test::{Expect, expect};
use indoc::indoc;

mod common;

fn check(input: &str, expect: Expect) {
    let actual = common::process_via_library(input);
    let sorted = common::sorted_output(&actual);
    expect.assert_eq(&sorted);
}

#[test]
fn smoke_basic() {
    check(
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

/// Verifies whitespace tolerance (spaces after commas) and insufficient funds.
/// Client 2's withdrawal of 4.0 fails because they only have 2.5.
#[test]
fn smoke_whitespace_and_insufficient_funds() {
    check(
        indoc! {"
            type, client, tx, amount
            deposit, 1, 1, 1.25
            deposit, 2, 2, 2.5
            deposit, 1, 3, 2.25
            withdrawal, 1, 4, 1.75
            withdrawal, 2, 5, 4.0
        "},
        expect![[r#"
            client,available,held,total,locked
            1,1.7500,0.0000,1.7500,false
            2,2.5000,0.0000,2.5000,false
        "#]],
    );
}

#[test]
fn smoke_empty() {
    check(
        indoc! {"
            type,client,tx,amount
        "},
        expect![[r#"
            client,available,held,total,locked
        "#]],
    );
}
