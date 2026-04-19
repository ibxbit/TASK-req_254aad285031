//! Search params + URL builder. Mirrors what the frontend POSTs in
//! `api::catalog::search` so the query string construction is verified
//! end-to-end without needing a browser.
//!
//! Supported filters (backend-recognised query params):
//!   q, min_price, max_price, min_rating,
//!   user_zip, sort, available_from, available_to,
//!   categories (comma-separated uuid list),
//!   tags (comma-separated uuid list),
//!   limit, offset.

use crate::url::urlencode;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct SearchParams {
    pub q: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub min_rating: Option<f64>,
    pub user_zip: Option<String>,
    pub sort: Option<String>,

    // Availability window (server parses `%Y-%m-%dT%H:%M:%S`).
    pub available_from: Option<String>,
    pub available_to: Option<String>,

    // Category filter — service must belong to ALL listed category IDs.
    // Empty/None -> no filter.
    pub categories: Vec<String>,
    // Tag filter — service must carry ANY listed tag ID.
    // Empty/None -> no filter.
    pub tags: Vec<String>,

    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub fn build_search_path(p: &SearchParams) -> String {
    let mut qs: Vec<String> = Vec::new();
    if let Some(v) = &p.q {
        if !v.trim().is_empty() {
            qs.push(format!("q={}", urlencode(v)));
        }
    }
    if let Some(v) = p.min_price {
        qs.push(format!("min_price={v}"));
    }
    if let Some(v) = p.max_price {
        qs.push(format!("max_price={v}"));
    }
    if let Some(v) = p.min_rating {
        qs.push(format!("min_rating={v}"));
    }
    if let Some(v) = &p.user_zip {
        if !v.trim().is_empty() {
            qs.push(format!("user_zip={}", urlencode(v)));
        }
    }
    if let Some(v) = &p.available_from {
        if !v.trim().is_empty() {
            qs.push(format!("available_from={}", urlencode(v)));
        }
    }
    if let Some(v) = &p.available_to {
        if !v.trim().is_empty() {
            qs.push(format!("available_to={}", urlencode(v)));
        }
    }
    // Comma-joined UUID lists. Blank ids are dropped so an empty form
    // field doesn't send `categories=,`.
    let cats = join_ids(&p.categories);
    if !cats.is_empty() {
        qs.push(format!("categories={}", urlencode(&cats)));
    }
    let tags = join_ids(&p.tags);
    if !tags.is_empty() {
        qs.push(format!("tags={}", urlencode(&tags)));
    }
    if let Some(v) = p.limit {
        qs.push(format!("limit={v}"));
    }
    if let Some(v) = p.offset {
        qs.push(format!("offset={v}"));
    }
    if let Some(v) = &p.sort {
        qs.push(format!("sort={v}"));
    }
    if qs.is_empty() {
        "/api/services/search".to_string()
    } else {
        format!("/api/services/search?{}", qs.join("&"))
    }
}

fn join_ids(ids: &[String]) -> String {
    ids.iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_params_return_bare_path() {
        assert_eq!(
            build_search_path(&SearchParams::default()),
            "/api/services/search"
        );
    }

    #[test]
    fn blank_query_string_is_not_emitted() {
        // Real UX: user clears the search box but still wants filters — the
        // empty `q` must not land in the URL as `q=`.
        let p = SearchParams {
            q: Some("   ".into()),
            min_price: Some(1.0),
            ..Default::default()
        };
        let path = build_search_path(&p);
        assert!(!path.contains("q="), "blank q must not appear: {path}");
        assert!(path.contains("min_price=1"));
    }

    #[test]
    fn sort_and_numeric_filters_compose() {
        let p = SearchParams {
            min_price: Some(10.0),
            max_price: Some(100.0),
            min_rating: Some(4.5),
            sort: Some("best_rated".into()),
            ..Default::default()
        };
        let path = build_search_path(&p);
        assert!(path.starts_with("/api/services/search?"));
        for piece in &[
            "min_price=10",
            "max_price=100",
            "min_rating=4.5",
            "sort=best_rated",
        ] {
            assert!(path.contains(piece), "missing {piece} in {path}");
        }
    }

    #[test]
    fn query_with_special_chars_is_encoded() {
        let p = SearchParams {
            q: Some("pipe & fitter".into()),
            ..Default::default()
        };
        let path = build_search_path(&p);
        assert!(path.contains("q=pipe%20%26%20fitter"), "got {path}");
    }

    #[test]
    fn user_zip_trimmed_whitespace_is_dropped() {
        let p = SearchParams {
            user_zip: Some("   ".into()),
            ..Default::default()
        };
        assert_eq!(build_search_path(&p), "/api/services/search");
    }

    #[test]
    fn user_zip_non_blank_is_encoded() {
        let p = SearchParams {
            user_zip: Some("94110".into()),
            ..Default::default()
        };
        assert_eq!(build_search_path(&p), "/api/services/search?user_zip=94110");
    }

    // --- Availability window ---

    #[test]
    fn availability_window_both_ends_are_emitted_and_encoded() {
        let p = SearchParams {
            available_from: Some("2026-06-01T10:00:00".into()),
            available_to: Some("2026-06-01T12:00:00".into()),
            ..Default::default()
        };
        let path = build_search_path(&p);
        // Colons must be percent-encoded.
        assert!(
            path.contains("available_from=2026-06-01T10%3A00%3A00"),
            "encoded from missing: {path}"
        );
        assert!(
            path.contains("available_to=2026-06-01T12%3A00%3A00"),
            "encoded to missing: {path}"
        );
    }

    #[test]
    fn empty_availability_fields_are_dropped() {
        let p = SearchParams {
            available_from: Some("   ".into()),
            available_to: Some("".into()),
            ..Default::default()
        };
        assert_eq!(build_search_path(&p), "/api/services/search");
    }

    // --- Category + tag filter ---

    #[test]
    fn category_and_tag_filters_join_ids_with_commas() {
        let p = SearchParams {
            categories: vec!["c1".into(), "c2".into()],
            tags: vec!["t1".into()],
            ..Default::default()
        };
        let path = build_search_path(&p);
        assert!(
            path.contains("categories=c1%2Cc2"),
            "categories missing: {path}"
        );
        assert!(path.contains("tags=t1"), "tag missing: {path}");
    }

    #[test]
    fn blank_entries_in_category_or_tag_list_are_dropped() {
        let p = SearchParams {
            categories: vec!["".into(), "c1".into(), "  ".into()],
            tags: vec!["  ".into()],
            ..Default::default()
        };
        let path = build_search_path(&p);
        assert!(path.contains("categories=c1"), "expected just c1: {path}");
        assert!(
            !path.contains("tags="),
            "all-blank tags must be dropped: {path}"
        );
    }

    // --- Pagination ---

    #[test]
    fn pagination_limit_and_offset_are_emitted() {
        let p = SearchParams {
            limit: Some(20),
            offset: Some(40),
            ..Default::default()
        };
        let path = build_search_path(&p);
        assert!(path.contains("limit=20"));
        assert!(path.contains("offset=40"));
    }

    #[test]
    fn full_composite_filter_has_expected_params() {
        let p = SearchParams {
            q: Some("heat pump".into()),
            min_price: Some(5.0),
            max_price: Some(500.0),
            min_rating: Some(3.0),
            user_zip: Some("94110".into()),
            sort: Some("soonest_available".into()),
            available_from: Some("2026-06-01T10:00:00".into()),
            available_to: Some("2026-06-02T10:00:00".into()),
            categories: vec!["cat-a".into()],
            tags: vec!["tag-a".into(), "tag-b".into()],
            limit: Some(25),
            offset: Some(0),
        };
        let path = build_search_path(&p);
        for seg in &[
            "q=heat%20pump",
            "min_price=5",
            "max_price=500",
            "min_rating=3",
            "user_zip=94110",
            "available_from=2026-06-01T10%3A00%3A00",
            "available_to=2026-06-02T10%3A00%3A00",
            "categories=cat-a",
            "tags=tag-a%2Ctag-b",
            "limit=25",
            "offset=0",
            "sort=soonest_available",
        ] {
            assert!(path.contains(seg), "missing `{seg}` in `{path}`");
        }
    }
}
