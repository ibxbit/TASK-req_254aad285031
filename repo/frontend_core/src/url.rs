//! URL encoding for query-string values. RFC 3986 unreserved set
//! (alphanumerics + `-` `_` `.` `~`) passes through; everything else is
//! percent-escaped. Used by search params construction on the frontend.

pub fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alphanumerics_unchanged() {
        assert_eq!(urlencode("abcXYZ0123"), "abcXYZ0123");
    }

    #[test]
    fn unreserved_symbols_pass_through() {
        assert_eq!(urlencode("A_b-c.d~e"), "A_b-c.d~e");
    }

    #[test]
    fn space_becomes_percent_20() {
        assert_eq!(urlencode("hello world"), "hello%20world");
    }

    #[test]
    fn ampersand_and_equals_are_escaped() {
        assert_eq!(urlencode("a=b&c=d"), "a%3Db%26c%3Dd");
    }

    #[test]
    fn unicode_is_utf8_percent_encoded() {
        // "é" is 0xC3 0xA9 in UTF-8.
        assert_eq!(urlencode("é"), "%C3%A9");
    }

    #[test]
    fn empty_string_is_empty() {
        assert_eq!(urlencode(""), "");
    }
}
