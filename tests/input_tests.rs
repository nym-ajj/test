//! Input handling tests - malformed data, whitespace, edge cases.

use expect_test::{Expect, expect};
use indoc::indoc;

mod common;

fn check(input: &str, expect: Expect) {
    let actual = common::process_via_library(input);
    let sorted = common::sorted_output(&actual);
    expect.assert_eq(&sorted);
}

#[test]
fn input_malformed() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit,1,1,100.0
            invalid_type,1,2,50.0
            deposit,abc,3,25.0
            deposit,2,4,75.0
            ,3,5,10.0
            deposit,3,6,
        "},
        expect![[r#"
            client,available,held,total,locked
            1,100.0000,0.0000,100.0000,false
            2,75.0000,0.0000,75.0000,false
        "#]],
    );
}

#[test]
fn input_whitespace() {
    check(
        indoc! {"
            type,client,tx,amount
            deposit , 1 , 1 , 100.0
            withdrawal, 1, 2, 25.0
            deposit,2,3,  50.0  
        "},
        expect![[r#"
            client,available,held,total,locked
            1,75.0000,0.0000,75.0000,false
            2,50.0000,0.0000,50.0000,false
        "#]],
    );
}
