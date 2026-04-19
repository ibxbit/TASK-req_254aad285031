//! Tag selection behavior: clicking a tag in the review form toggles its
//! presence in the submitted `tag_ids` list. Extracted so we can assert
//! the exact vector mutation without booting a Dioxus runtime.

pub fn toggle_tag(selected: &mut Vec<String>, tag_id: &str) {
    if let Some(pos) = selected.iter().position(|x| x == tag_id) {
        selected.remove(pos);
    } else {
        selected.push(tag_id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling_absent_tag_adds_it() {
        let mut v: Vec<String> = vec![];
        toggle_tag(&mut v, "a");
        assert_eq!(v, vec!["a".to_string()]);
    }

    #[test]
    fn toggling_present_tag_removes_it() {
        let mut v: Vec<String> = vec!["a".into(), "b".into()];
        toggle_tag(&mut v, "a");
        assert_eq!(v, vec!["b".to_string()]);
    }

    #[test]
    fn toggling_is_idempotent_over_two_clicks() {
        let mut v: Vec<String> = vec!["x".into()];
        toggle_tag(&mut v, "y");
        toggle_tag(&mut v, "y");
        assert_eq!(v, vec!["x".to_string()]);
    }

    #[test]
    fn toggle_preserves_order_of_other_tags() {
        let mut v: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        toggle_tag(&mut v, "b");
        assert_eq!(v, vec!["a".to_string(), "c".to_string()]);
    }
}
