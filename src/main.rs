fn main() {
    let result = add(2, 2);
    assert_eq!(result, 4);
}

fn add(a: i32, b: i32) -> i32 {
    a.saturating_add(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(1, 1), 2);
        assert_eq!(add(0, 0), 0);
        assert_eq!(add(-1, 1), 0);
    }
}
