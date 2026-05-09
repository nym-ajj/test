//! Adversarial tests - negative amounts, duplicates, locked accounts, fraud
//! patterns.

use expect_test::{Expect, expect};
use indoc::indoc;

mod common;

fn check(input: &str, expect: Expect) {
    let actual = common::process_via_library(input);
    let sorted = common::sorted_output(&actual);
    expect.assert_eq(&sorted);
}

#[test]
fn adversarial_negative() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            deposit,1,2,-50.0
            withdrawal,1,3,-25.0
            withdrawal,1,4,25.0
        "},
        expect![[r#"
            client,available,held,total,locked
            1,75.0000,0.0000,75.0000,false
        "#]],
    );
}

#[test]
fn adversarial_duplicates() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            deposit,2,1,50.0
            deposit,1,2,25.0
        "},
        expect![[r#"
            client,available,held,total,locked
            1,125.0000,0.0000,125.0000,false
        "#]],
    );
}

#[test]
fn adversarial_locked() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            dispute,1,1,
            chargeback,1,1,
            deposit,1,2,50.0
            withdrawal,1,3,25.0
            dispute,1,1,
        "},
        expect![[r#"
            client,available,held,total,locked
            1,0.0000,0.0000,0.0000,true
        "#]],
    );
}

#[test]
fn adversarial_fraud() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            withdrawal,1,2,100.0
            dispute,1,1,
        "},
        expect![[r#"
            client,available,held,total,locked
            1,-100.0000,100.0000,0.0000,false
        "#]],
    );
}
