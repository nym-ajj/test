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

/// Tests the exact example from the spec (rust-coding-challenge.md).
/// Input has spaces after commas to verify whitespace handling.
/// Client 2's withdrawal of 3.0 fails (insufficient funds: only has 2.0).
#[test]
fn smoke_spec_example() {
    check(
        indoc! {"
            type, client, tx, amount
            deposit, 1, 1, 1.0
            deposit, 2, 2, 2.0
            deposit, 1, 3, 2.0
            withdrawal, 1, 4, 1.5
            withdrawal, 2, 5, 3.0
        "},
        expect![[r#"
            client,available,held,total,locked
            1,1.5000,0.0000,1.5000,false
            2,2.0000,0.0000,2.0000,false
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
