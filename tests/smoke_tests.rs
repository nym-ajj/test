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
