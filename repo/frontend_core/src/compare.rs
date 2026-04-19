//! Compare-selection state logic. Extracted from the Dioxus Catalog page so
//! the behavior (toggle up to a hard limit of 3) is verifiable natively
//! without a browser.

/// Maximum number of services a user may select for side-by-side comparison.
/// Must match the `COMPARE_LIMIT` used in `frontend/src/pages/catalog.rs`.
pub const COMPARE_LIMIT: usize = 3;

/// Toggle `id` in `selected`:
/// - If already present, remove it.
/// - If not present AND `selected.len() < COMPARE_LIMIT`, append it.
/// - If not present AND already at the limit, silently ignore.
pub fn toggle_compare(selected: &mut Vec<String>, id: &str) {
    if let Some(pos) = selected.iter().position(|s| s == id) {
        selected.remove(pos);
    } else if selected.len() < COMPARE_LIMIT {
        selected.push(id.to_string());
    }
}

/// Returns `true` when `selected` has reached `COMPARE_LIMIT`.
/// The UI uses this to disable the per-service checkbox.
pub fn at_limit(selected: &[String]) -> bool {
    selected.len() >= COMPARE_LIMIT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_limit_constant_is_three() {
        assert_eq!(COMPARE_LIMIT, 3);
    }

    #[test]
    fn adding_first_id_succeeds() {
        let mut sel = Vec::new();
        toggle_compare(&mut sel, "a");
        assert_eq!(sel, vec!["a"]);
    }

    #[test]
    fn adding_up_to_limit_succeeds() {
        let mut sel = Vec::new();
        toggle_compare(&mut sel, "a");
        toggle_compare(&mut sel, "b");
        toggle_compare(&mut sel, "c");
        assert_eq!(sel.len(), COMPARE_LIMIT);
    }

    #[test]
    fn adding_beyond_limit_is_silently_ignored() {
        let mut sel = vec!["a".into(), "b".into(), "c".into()];
        toggle_compare(&mut sel, "d");
        assert_eq!(sel.len(), COMPARE_LIMIT, "must not exceed limit");
        assert!(!sel.contains(&"d".to_string()), "d must not be added");
    }

    #[test]
    fn removing_a_present_id_shrinks_selection() {
        let mut sel = vec!["a".into(), "b".into(), "c".into()];
        toggle_compare(&mut sel, "b");
        assert_eq!(sel, vec!["a", "c"]);
    }

    #[test]
    fn removing_allows_adding_a_new_id_after_limit() {
        let mut sel = vec!["a".into(), "b".into(), "c".into()];
        toggle_compare(&mut sel, "a"); // remove
        toggle_compare(&mut sel, "d"); // now below limit → add
        assert!(sel.contains(&"d".to_string()));
        assert!(!sel.contains(&"a".to_string()));
    }

    #[test]
    fn at_limit_false_when_below_limit() {
        let sel: Vec<String> = vec!["a".into(), "b".into()];
        assert!(!at_limit(&sel));
    }

    #[test]
    fn at_limit_true_when_at_limit() {
        let sel: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert!(at_limit(&sel));
    }

    #[test]
    fn at_limit_true_when_empty_and_limit_is_zero_edge() {
        let sel: Vec<String> = Vec::new();
        assert!(!at_limit(&sel), "empty is not at limit");
    }

    #[test]
    fn toggle_is_idempotent_over_add_then_remove() {
        let mut sel = Vec::new();
        toggle_compare(&mut sel, "x");
        toggle_compare(&mut sel, "x");
        assert!(sel.is_empty(), "add then remove must yield empty");
    }
}
