//! Edge case tests - zero amounts, boundary values.

use expect_test::{Expect, expect};
use indoc::indoc;

mod common;

fn check(input: &str, expect: Expect) {
    let actual = common::process_via_library(input);
    let sorted = common::sorted_output(&actual);
    expect.assert_eq(&sorted);
}

#[test]
fn edge_zero_amount() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,0.0
            deposit,1,2,100.0
            withdrawal,1,3,0.0
        "},
        expect![[r#"
            client,available,held,total,locked
            1,100.0000,0.0000,100.0000,false
        "#]],
    );
}

#[test]
fn edge_boundaries() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,65535,4294967295,0.0001
            deposit,65535,1,99.9999
        "},
        expect![[r#"
            client,available,held,total,locked
            65535,100.0000,0.0000,100.0000,false
        "#]],
    );
}
