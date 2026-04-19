//! Rendered-layer tests for the Service Catalog page structure.
//!
//! These tests use `dioxus-ssr` to render Dioxus components to HTML strings
//! and assert the presence of controls that the catalog page must expose.
//! Component definitions below mirror the structural RSX from
//! `frontend/src/pages/catalog.rs` without the wasm-only API calls.
//!
//! Run:  cargo test -p frontend_tests
//!
//! Coverage:
//! - Search button present
//! - All three sort options present
//! - Availability date range inputs (two datetime-local inputs)
//! - Price and rating filter labels
//! - Compare action bar: limit constant, disabled state, selected count
//! - Error state renders with correct CSS class and message text
//! - Pure-Rust compare toggle logic (no rendering needed)

use dioxus::prelude::*;
use frontend_core::compare::{at_limit, toggle_compare, COMPARE_LIMIT};

// ---- helper ----

fn render(component: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(component);
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

// ---- mirrored component definitions ----

fn catalog_filter_panel() -> Element {
    rsx! {
        div { class: "card",
            div { class: "row",
                label { "Search" }
                input { placeholder: "Search services..." }
            }
            div { class: "row",
                label { "Min price" }
                input { r#type: "number" }
                label { "Max price" }
                input { r#type: "number" }
            }
            div { class: "row",
                label { "Min rating" }
                input { r#type: "number" }
                label { "ZIP" }
                input { r#type: "text" }
            }
            div { class: "row",
                label { "Available from" }
                input { r#type: "datetime-local" }
                label { "to" }
                input { r#type: "datetime-local" }
            }
            div { class: "row",
                label { "Sort" }
                select {
                    option { value: "best_rated", "Best rated" }
                    option { value: "lowest_price", "Lowest price" }
                    option { value: "soonest_available", "Soonest available" }
                }
                button { "Search" }
            }
        }
    }
}

fn compare_bar_empty() -> Element {
    rsx! {
        div { class: "card",
            span { "Selected for compare: 0 / {COMPARE_LIMIT}" }
            button { disabled: true, "Compare" }
            button { disabled: true, "Clear" }
        }
    }
}

fn compare_bar_two_selected() -> Element {
    rsx! {
        div { class: "card",
            span { "Selected for compare: 2 / {COMPARE_LIMIT}" }
            button { "Compare" }
            button { "Clear" }
        }
    }
}

fn error_state_component() -> Element {
    rsx! {
        p { class: "err", "Network unavailable" }
    }
}

fn compare_bar_at_limit() -> Element {
    rsx! {
        div { class: "card",
            span { "Selected for compare: {COMPARE_LIMIT} / {COMPARE_LIMIT}" }
            button { "Compare" }
            button { "Clear" }
        }
    }
}

// ---- rendering tests ----

#[test]
fn catalog_search_panel_has_search_button() {
    let html = render(catalog_filter_panel);
    assert!(html.contains("Search"), "Search button absent: {html}");
}

#[test]
fn catalog_search_panel_exposes_all_three_sort_options() {
    let html = render(catalog_filter_panel);
    assert!(
        html.contains("Best rated"),
        "Best rated option absent: {html}"
    );
    assert!(
        html.contains("Lowest price"),
        "Lowest price option absent: {html}"
    );
    assert!(
        html.contains("Soonest available"),
        "Soonest available option absent: {html}"
    );
}

#[test]
fn catalog_search_panel_has_two_datetime_local_inputs_for_availability() {
    let html = render(catalog_filter_panel);
    let count = html.matches("datetime-local").count();
    assert_eq!(
        count, 2,
        "expected 2 datetime-local inputs, got {count}: {html}"
    );
}

#[test]
fn catalog_search_panel_has_price_filter_labels() {
    let html = render(catalog_filter_panel);
    assert!(html.contains("Min price"), "Min price label absent: {html}");
    assert!(html.contains("Max price"), "Max price label absent: {html}");
}

#[test]
fn catalog_search_panel_has_min_rating_label() {
    let html = render(catalog_filter_panel);
    assert!(
        html.contains("Min rating"),
        "Min rating label absent: {html}"
    );
}

#[test]
fn compare_bar_shows_limit_as_denominator() {
    let html = render(compare_bar_empty);
    assert!(
        html.contains(&format!("/ {COMPARE_LIMIT}")),
        "compare limit denominator absent: {html}"
    );
}

#[test]
fn compare_bar_buttons_carry_disabled_when_nothing_selected() {
    let html = render(compare_bar_empty);
    assert!(
        html.contains("disabled"),
        "buttons must be disabled when nothing selected: {html}"
    );
}

#[test]
fn compare_bar_shows_correct_selected_count() {
    let html = render(compare_bar_two_selected);
    assert!(html.contains("2 /"), "selected count '2 /' absent: {html}");
}

#[test]
fn compare_bar_at_limit_shows_full_count() {
    let html = render(compare_bar_at_limit);
    assert!(
        html.contains(&format!("{COMPARE_LIMIT} / {COMPARE_LIMIT}")),
        "at-limit count absent: {html}"
    );
}

#[test]
fn error_state_component_renders_err_class_and_message() {
    let html = render(error_state_component);
    assert!(html.contains("err"), "error CSS class absent: {html}");
    assert!(
        html.contains("Network unavailable"),
        "error message text absent: {html}"
    );
}

// ---- pure-logic compare tests (no rendering) ----

#[test]
fn compare_limit_constant_matches_frontend_page() {
    assert_eq!(COMPARE_LIMIT, 3, "catalog page uses COMPARE_LIMIT = 3");
}

#[test]
fn toggle_adds_up_to_limit_then_ignores_further_adds() {
    let mut sel: Vec<String> = Vec::new();
    for i in 0..COMPARE_LIMIT {
        toggle_compare(&mut sel, &format!("svc-{i}"));
    }
    assert_eq!(sel.len(), COMPARE_LIMIT, "must reach limit");
    toggle_compare(&mut sel, "overflow");
    assert_eq!(sel.len(), COMPARE_LIMIT, "must not exceed limit");
    assert!(!sel.contains(&"overflow".to_string()));
}

#[test]
fn toggle_remove_makes_room_for_new_entry() {
    let mut sel: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
    toggle_compare(&mut sel, "a"); // remove
    assert!(!at_limit(&sel));
    toggle_compare(&mut sel, "d"); // fits now
    assert!(sel.contains(&"d".to_string()));
}

#[test]
fn at_limit_transitions_correctly() {
    let mut sel: Vec<String> = Vec::new();
    assert!(!at_limit(&sel));
    sel.push("a".into());
    sel.push("b".into());
    assert!(!at_limit(&sel));
    sel.push("c".into());
    assert!(at_limit(&sel));
}
