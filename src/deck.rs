/// Wraps a deck index: next from the last card goes to the first, prev from the first to the last.
pub fn wrap_index(i: i32, d: i32, n: i32) -> i32 {
    if n == 0 {
        return 0;
    }
    let mut x = (i + d) % n;
    if x < 0 {
        x += n;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_next_from_last_to_first() {
        assert_eq!(wrap_index(3, 1, 4), 0);
    }

    #[test]
    fn wraps_prev_from_first_to_last() {
        assert_eq!(wrap_index(0, -1, 4), 3);
    }

    #[test]
    fn moves_normally_in_between() {
        assert_eq!(wrap_index(1, 1, 4), 2);
    }
}
