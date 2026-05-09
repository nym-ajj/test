//! Core functionality tests - deposits, withdrawals, precision.

use expect_test::{Expect, expect};
use indoc::indoc;

mod common;

fn check(input: &str, expect: Expect) {
    let actual = common::process_via_library(input);
    let sorted = common::sorted_output(&actual);
    expect.assert_eq(&sorted);
}

#[test]
fn core_deposit_withdrawal() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            withdrawal,1,2,30.0
            deposit,1,3,50.0
            withdrawal,1,4,20.0
        "},
        expect![[r#"
            client,available,held,total,locked
            1,100.0000,0.0000,100.0000,false
        "#]],
    );
}

#[test]
fn core_multiple_clients() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            deposit,2,2,200.0
            withdrawal,1,3,50.0
            deposit,3,4,300.0
            withdrawal,2,5,100.0
        "},
        expect![[r#"
            client,available,held,total,locked
            1,50.0000,0.0000,50.0000,false
            2,100.0000,0.0000,100.0000,false
            3,300.0000,0.0000,300.0000,false
        "#]],
    );
}

#[test]
fn core_precision() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,1.2345
            withdrawal,1,2,0.1234
            deposit,1,3,0.0001
        "},
        expect![[r#"
            client,available,held,total,locked
            1,1.1112,0.0000,1.1112,false
        "#]],
    );
}

#[test]
fn core_insufficient_funds() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,50.0
            withdrawal,1,2,100.0
            withdrawal,1,3,25.0
        "},
        expect![[r#"
            client,available,held,total,locked
            1,25.0000,0.0000,25.0000,false
        "#]],
    );
}
