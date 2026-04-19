//! Rating input clamp. The backend rejects anything outside 1..=5 with
//! 400; the frontend pre-clamps so the user can't submit an obviously
//! invalid review.

pub fn clamp_rating(n: u8) -> u8 {
    if n < 1 {
        1
    } else if n > 5 {
        5
    } else {
        n
    }
}

pub fn parse_rating(raw: &str) -> Option<u8> {
    raw.trim().parse::<u8>().ok().map(clamp_rating)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_rejects_below_one() {
        assert_eq!(clamp_rating(0), 1);
    }

    #[test]
    fn clamp_rejects_above_five() {
        assert_eq!(clamp_rating(6), 5);
        assert_eq!(clamp_rating(200), 5);
    }

    #[test]
    fn clamp_passes_valid_range_through() {
        for n in 1..=5u8 {
            assert_eq!(clamp_rating(n), n);
        }
    }

    #[test]
    fn parse_rating_handles_trimmed_input() {
        assert_eq!(parse_rating("  3  "), Some(3));
    }

    #[test]
    fn parse_rating_clamps_out_of_range_input() {
        assert_eq!(parse_rating("9"), Some(5));
    }

    #[test]
    fn parse_rating_rejects_non_numeric() {
        assert_eq!(parse_rating("abc"), None);
        assert_eq!(parse_rating(""), None);
    }
}
