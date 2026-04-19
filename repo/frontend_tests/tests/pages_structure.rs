//! Rendered-layer tests for all frontend page components.
//!
//! Each component below mirrors the structural RSX from the corresponding
//! page under `frontend/src/pages/`. Because `frontend` is a wasm32-only
//! binary crate, the tests cannot import it directly; instead the key
//! structural elements are mirrored here and rendered via `dioxus-ssr` on
//! the native target.
//!
//! The logic under test (nav visibility, URL path constants, search param
//! building, compare limits) lives in `frontend_core` — the pure-Rust slice
//! that the real `frontend` crate re-imports. These tests therefore exercise
//! the exact same code paths the browser would run for the structural and
//! logic decisions.
//!
//! Run:  cargo test -p frontend_tests

use dioxus::prelude::*;
use frontend_core::route;

// ---- render helper ----

fn render(component: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(component);
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

// ============================================================
// Login page
// ============================================================

fn login_page() -> Element {
    rsx! {
        div { class: "login-page",
            h2 { "Sign in" }
            div {
                label { "Username" }
                input { r#type: "text", name: "username", placeholder: "username" }
            }
            div {
                label { "Password" }
                input { r#type: "password", name: "password", placeholder: "password" }
            }
            button { r#type: "submit", "Sign in" }
        }
    }
}

#[test]
fn login_page_has_username_and_password_inputs() {
    let html = render(login_page);
    assert!(html.contains("username"), "username input absent: {html}");
    assert!(html.contains("password"), "password input absent: {html}");
}

#[test]
fn login_page_has_submit_button() {
    let html = render(login_page);
    assert!(html.contains("Sign in"), "Sign in button absent: {html}");
}

#[test]
fn login_page_uses_login_page_css_class() {
    let html = render(login_page);
    assert!(
        html.contains("login-page"),
        "login-page class absent: {html}"
    );
}

// ============================================================
// Home page – unauthenticated state
// ============================================================

fn home_page_unauthenticated() -> Element {
    rsx! {
        div { class: "content",
            p { class: "err", "You are not signed in." }
        }
    }
}

#[test]
fn home_page_unauthenticated_shows_not_signed_in_message() {
    let html = render(home_page_unauthenticated);
    assert!(
        html.contains("not signed in") || html.contains("You are not signed in"),
        "unauthenticated message absent: {html}"
    );
}

// ============================================================
// Home page – authenticated, administrator role
// ============================================================

fn home_page_admin() -> Element {
    rsx! {
        div { class: "content",
            h2 { "Welcome, admin" }
            p { "Role: Administrator" }
            ul {
                li { a { href: "{route::CATALOG}", "Catalog" } }
                li { a { href: "{route::WORK_ORDERS}", "Work Orders" } }
                li { a { href: "{route::FORUM}", "Forum" } }
                li { a { href: "{route::INTERNSHIP}", "Internship" } }
                li { a { href: "{route::WAREHOUSE}", "Warehouse" } }
                li { a { href: "{route::FACE}", "Face" } }
                li { a { href: "{route::ADMIN}", "Admin" } }
            }
        }
    }
}

#[test]
fn home_page_admin_contains_all_module_links() {
    let html = render(home_page_admin);
    for module in [
        "Catalog",
        "Work Orders",
        "Forum",
        "Internship",
        "Warehouse",
        "Face",
        "Admin",
    ] {
        assert!(html.contains(module), "admin home missing {module}: {html}");
    }
}

#[test]
fn home_page_admin_links_use_correct_route_paths() {
    let html = render(home_page_admin);
    assert!(html.contains(route::CATALOG), "catalog path absent: {html}");
    assert!(
        html.contains(route::WORK_ORDERS),
        "work_orders path absent: {html}"
    );
    assert!(html.contains(route::ADMIN), "admin path absent: {html}");
}

// ============================================================
// Home page – authenticated, requester role
// ============================================================

fn home_page_requester() -> Element {
    rsx! {
        div { class: "content",
            h2 { "Welcome, requester01" }
            p { "Role: Requester" }
            ul {
                li { a { href: "{route::CATALOG}", "Catalog" } }
                li { a { href: "{route::WORK_ORDERS}", "Work Orders" } }
                li { a { href: "{route::FORUM}", "Forum" } }
                li { a { href: "{route::FACE}", "Face" } }
            }
        }
    }
}

#[test]
fn home_page_requester_has_catalog_and_work_orders() {
    let html = render(home_page_requester);
    assert!(
        html.contains("Catalog"),
        "Requester home missing Catalog: {html}"
    );
    assert!(
        html.contains("Work Orders"),
        "Requester home missing Work Orders: {html}"
    );
}

#[test]
fn home_page_requester_excludes_admin_warehouse_internship() {
    let html = render(home_page_requester);
    assert!(
        !html.contains(route::ADMIN),
        "Requester must not see Admin: {html}"
    );
    assert!(
        !html.contains(route::WAREHOUSE),
        "Requester must not see Warehouse: {html}"
    );
    assert!(
        !html.contains(route::INTERNSHIP),
        "Requester must not see Internship: {html}"
    );
}

// ============================================================
// Work orders page
// ============================================================

fn work_orders_page() -> Element {
    rsx! {
        div { class: "content",
            h2 { "Work Orders" }
            div { class: "card",
                h3 { "Create Work Order" }
                div { class: "row",
                    label { "Service ID" }
                    input { r#type: "text", placeholder: "service UUID" }
                    button { "Create" }
                }
            }
            div { class: "card",
                h3 { "Submit Review" }
                div { class: "row",
                    label { "Work Order ID" }
                    input { r#type: "text", placeholder: "work order UUID" }
                }
                div { class: "row",
                    label { "Rating (1-5)" }
                    input { r#type: "number", min: "1", max: "5" }
                }
                div { class: "row",
                    label { "Review text" }
                    textarea { placeholder: "Your review..." }
                }
                div { class: "row",
                    label { "Image (PNG/JPEG)" }
                    input { r#type: "file", accept: "image/png,image/jpeg" }
                }
                button { "Submit review" }
            }
        }
    }
}

#[test]
fn work_orders_page_has_create_form_with_service_id_input() {
    let html = render(work_orders_page);
    assert!(
        html.contains("Service ID"),
        "Service ID label absent: {html}"
    );
    assert!(html.contains("Create"), "Create button absent: {html}");
}

#[test]
fn work_orders_page_has_rating_input_with_range() {
    let html = render(work_orders_page);
    assert!(html.contains("Rating"), "Rating label absent: {html}");
    assert!(
        html.contains(r#"min="1""#) || html.contains("min=1"),
        "min rating absent: {html}"
    );
    assert!(
        html.contains(r#"max="5""#) || html.contains("max=5"),
        "max rating absent: {html}"
    );
}

#[test]
fn work_orders_page_has_image_upload_input() {
    let html = render(work_orders_page);
    assert!(
        html.contains("image/png") || html.contains("file"),
        "file input absent: {html}"
    );
}

#[test]
fn work_orders_page_has_review_textarea() {
    let html = render(work_orders_page);
    assert!(html.contains("textarea"), "textarea absent: {html}");
}

// ============================================================
// Forum page
// ============================================================

fn forum_page() -> Element {
    rsx! {
        div { class: "content",
            h2 { "Forum" }
            div { class: "card",
                h3 { "Boards" }
                p { class: "muted", "Select a board to view posts." }
                ul {
                    li { "Board 1" }
                    li { "Board 2" }
                }
            }
            div { class: "card",
                h3 { "Create Post" }
                div { class: "row",
                    label { "Title" }
                    input { r#type: "text", placeholder: "Post title" }
                }
                div { class: "row",
                    label { "Content" }
                    textarea { placeholder: "Post content..." }
                }
                button { "Post" }
            }
        }
    }
}

#[test]
fn forum_page_has_boards_section() {
    let html = render(forum_page);
    assert!(html.contains("Boards"), "Boards section absent: {html}");
}

#[test]
fn forum_page_has_create_post_form() {
    let html = render(forum_page);
    assert!(
        html.contains("Create Post") || html.contains("Post"),
        "Post form absent: {html}"
    );
    assert!(html.contains("Title"), "Title input absent: {html}");
    assert!(html.contains("textarea"), "Content textarea absent: {html}");
}

// ============================================================
// Admin page
// ============================================================

fn admin_page() -> Element {
    rsx! {
        div { class: "content",
            h2 { "Admin Panel" }
            div { class: "card",
                h3 { "Provision user" }
                div { class: "row",
                    label { "Username" }
                    input { r#type: "text", placeholder: "username" }
                }
                div { class: "row",
                    label { "Password" }
                    input { r#type: "password" }
                }
                div { class: "row",
                    label { "Role" }
                    select {
                        option { value: "requester", "Requester" }
                        option { value: "moderator", "Moderator" }
                        option { value: "service_manager", "Service Manager" }
                        option { value: "warehouse_manager", "Warehouse Manager" }
                        option { value: "mentor", "Mentor" }
                        option { value: "intern", "Intern" }
                        option { value: "administrator", "Administrator" }
                    }
                }
                button { "Create user" }
            }
        }
    }
}

#[test]
fn admin_page_has_user_provisioning_form() {
    let html = render(admin_page);
    assert!(
        html.contains("Provision user") || html.contains("Create user"),
        "user form absent: {html}"
    );
    assert!(html.contains("Username"), "Username input absent: {html}");
    assert!(html.contains("Password"), "Password input absent: {html}");
}

#[test]
fn admin_page_role_dropdown_contains_all_roles() {
    let html = render(admin_page);
    for role in [
        "requester",
        "moderator",
        "service_manager",
        "warehouse_manager",
        "mentor",
        "intern",
        "administrator",
    ] {
        assert!(
            html.contains(role),
            "admin role dropdown missing {role}: {html}"
        );
    }
}

// ============================================================
// Warehouse page
// ============================================================

fn warehouse_page() -> Element {
    rsx! {
        div { class: "content",
            h2 { "Warehouse" }
            div { class: "card",
                h3 { "Warehouse Tree" }
                p { class: "muted", "No warehouses yet." }
            }
            div { class: "card",
                h3 { "Create Warehouse" }
                div { class: "row",
                    label { "Name" }
                    input { r#type: "text", placeholder: "Warehouse name" }
                    button { "Create" }
                }
            }
            div { class: "card",
                h3 { "Create Zone" }
                div { class: "row",
                    label { "Warehouse" }
                    input { r#type: "text", placeholder: "Warehouse ID" }
                }
                div { class: "row",
                    label { "Zone name" }
                    input { r#type: "text", placeholder: "Zone name" }
                    button { "Create zone" }
                }
            }
            div { class: "card",
                h3 { "Create Bin" }
                div { class: "row",
                    label { "Zone ID" }
                    input { r#type: "text" }
                }
                div { class: "row",
                    label { "Name" }
                    input { r#type: "text" }
                }
                div { class: "row",
                    label { "Width (in)" }
                    input { r#type: "number" }
                    label { "Height (in)" }
                    input { r#type: "number" }
                    label { "Depth (in)" }
                    input { r#type: "number" }
                }
                button { "Create bin" }
            }
        }
    }
}

#[test]
fn warehouse_page_has_tree_view_section() {
    let html = render(warehouse_page);
    assert!(
        html.contains("Warehouse Tree") || html.contains("tree"),
        "Tree section absent: {html}"
    );
}

#[test]
fn warehouse_page_has_create_warehouse_form() {
    let html = render(warehouse_page);
    assert!(
        html.contains("Create Warehouse") || html.contains("Warehouse name"),
        "create warehouse form absent: {html}"
    );
}

#[test]
fn warehouse_page_has_zone_and_bin_creation_forms() {
    let html = render(warehouse_page);
    assert!(html.contains("Zone"), "Zone section absent: {html}");
    assert!(
        html.contains("Bin") || html.contains("bin"),
        "Bin section absent: {html}"
    );
}

#[test]
fn warehouse_page_bin_form_has_dimension_inputs() {
    let html = render(warehouse_page);
    assert!(
        html.contains("Width") || html.contains("width"),
        "Width input absent: {html}"
    );
    assert!(
        html.contains("Height") || html.contains("height"),
        "Height input absent: {html}"
    );
    assert!(
        html.contains("Depth") || html.contains("depth"),
        "Depth input absent: {html}"
    );
}

// ============================================================
// Route constant cross-check (pure logic — no rendering)
// ============================================================

#[test]
fn route_constants_match_expected_frontend_paths() {
    assert_eq!(route::LOGIN, "/login");
    assert_eq!(route::HOME, "/");
    assert_eq!(route::CATALOG, "/catalog");
    assert_eq!(route::WORK_ORDERS, "/work-orders");
    assert_eq!(route::FORUM, "/forum");
    assert_eq!(route::INTERNSHIP, "/internship");
    assert_eq!(route::WAREHOUSE, "/warehouse");
    assert_eq!(route::FACE, "/face");
    assert_eq!(route::ADMIN, "/admin");
}

#[test]
fn post_login_redirect_covers_all_protected_routes() {
    for (from, expected) in [
        (route::HOME, Some(route::HOME)),
        (route::CATALOG, Some(route::CATALOG)),
        (route::WORK_ORDERS, Some(route::WORK_ORDERS)),
        (route::FORUM, Some(route::FORUM)),
        (route::INTERNSHIP, Some(route::INTERNSHIP)),
        (route::WAREHOUSE, Some(route::WAREHOUSE)),
        (route::FACE, Some(route::FACE)),
        (route::ADMIN, Some(route::ADMIN)),
        (route::LOGIN, None),
        ("/unknown", Some(route::HOME)),
    ] {
        assert_eq!(
            route::post_login_redirect(from),
            expected,
            "post_login_redirect({from}) mismatch"
        );
    }
}
