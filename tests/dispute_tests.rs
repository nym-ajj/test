//! Dispute flow tests - resolve, chargeback, re-dispute, cross-client.

use expect_test::{Expect, expect};
use indoc::indoc;

mod common;

fn check(input: &str, expect: Expect) {
    let actual = common::process_via_library(input);
    let sorted = common::sorted_output(&actual);
    expect.assert_eq(&sorted);
}

#[test]
fn dispute_resolve() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            dispute,1,1,
            resolve,1,1,
        "},
        expect![[r#"
            client,available,held,total,locked
            1,100.0000,0.0000,100.0000,false
        "#]],
    );
}

#[test]
fn dispute_chargeback() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            dispute,1,1,
            chargeback,1,1,
        "},
        expect![[r#"
            client,available,held,total,locked
            1,0.0000,0.0000,0.0000,true
        "#]],
    );
}

#[test]
fn dispute_redispute() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            dispute,1,1,
            resolve,1,1,
            dispute,1,1,
        "},
        expect![[r#"
            client,available,held,total,locked
            1,0.0000,100.0000,100.0000,false
        "#]],
    );
}

#[test]
fn dispute_cross_client() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            deposit,2,2,50.0
            dispute,2,1,
        "},
        expect![[r#"
            client,available,held,total,locked
            1,100.0000,0.0000,100.0000,false
            2,50.0000,0.0000,50.0000,false
        "#]],
    );
}

#[test]
fn dispute_nonexistent_tx() {
    // Disputing a transaction that doesn't exist is silently ignored
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            dispute,1,999,
        "},
        expect![[r#"
            client,available,held,total,locked
            1,100.0000,0.0000,100.0000,false
        "#]],
    );
}

#[test]
fn resolve_not_disputed() {
    // Resolving a transaction that isn't disputed is silently ignored
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            resolve,1,1,
        "},
        expect![[r#"
            client,available,held,total,locked
            1,100.0000,0.0000,100.0000,false
        "#]],
    );
}

#[test]
fn chargeback_not_disputed() {
    // Charging back a transaction that isn't disputed is silently ignored
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            chargeback,1,1,
        "},
        expect![[r#"
            client,available,held,total,locked
            1,100.0000,0.0000,100.0000,false
        "#]],
    );
}

#[test]
fn dispute_already_disputed() {
    // Disputing an already-disputed transaction is silently ignored
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            dispute,1,1,
            dispute,1,1,
        "},
        expect![[r#"
            client,available,held,total,locked
            1,0.0000,100.0000,100.0000,false
        "#]],
    );
}
