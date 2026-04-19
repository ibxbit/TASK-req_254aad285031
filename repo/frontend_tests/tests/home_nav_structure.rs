//! Rendered-layer tests for navigation visibility per role.
//!
//! Uses `dioxus-ssr` to render a nav list component for each role and asserts
//! that the correct modules appear (or are absent). The `menu_for` function
//! comes from `frontend_core::nav`, which is the single source of truth for
//! nav visibility shared by the real Dioxus layout component.
//!
//! Run:  cargo test -p frontend_tests

use dioxus::prelude::*;
use frontend_core::nav::{menu_for, NavItem};
use shared::Role;

// ---- render helper ----

fn render(component: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(component);
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

// ---- per-role nav components ----

fn nav_admin() -> Element {
    let items = menu_for(Role::Administrator);
    rsx! {
        ul {
            for item in items.into_iter() {
                li { "{item:?}" }
            }
        }
    }
}

fn nav_requester() -> Element {
    let items = menu_for(Role::Requester);
    rsx! {
        ul {
            for item in items.into_iter() {
                li { "{item:?}" }
            }
        }
    }
}

fn nav_intern() -> Element {
    let items = menu_for(Role::Intern);
    rsx! {
        ul {
            for item in items.into_iter() {
                li { "{item:?}" }
            }
        }
    }
}

fn nav_warehouse_manager() -> Element {
    let items = menu_for(Role::WarehouseManager);
    rsx! {
        ul {
            for item in items.into_iter() {
                li { "{item:?}" }
            }
        }
    }
}

fn nav_moderator() -> Element {
    let items = menu_for(Role::Moderator);
    rsx! {
        ul {
            for item in items.into_iter() {
                li { "{item:?}" }
            }
        }
    }
}

fn nav_mentor() -> Element {
    let items = menu_for(Role::Mentor);
    rsx! {
        ul {
            for item in items.into_iter() {
                li { "{item:?}" }
            }
        }
    }
}

fn nav_service_manager() -> Element {
    let items = menu_for(Role::ServiceManager);
    rsx! {
        ul {
            for item in items.into_iter() {
                li { "{item:?}" }
            }
        }
    }
}

// ---- tests ----

#[test]
fn admin_nav_renders_all_modules() {
    let html = render(nav_admin);
    for item in [
        "Home",
        "Catalog",
        "WorkOrders",
        "Forum",
        "Internship",
        "Warehouse",
        "Face",
        "Admin",
    ] {
        assert!(html.contains(item), "Admin nav missing {item}: {html}");
    }
}

#[test]
fn requester_nav_has_catalog_and_work_orders_not_admin_or_warehouse() {
    let html = render(nav_requester);
    assert!(
        html.contains("Catalog"),
        "Requester nav missing Catalog: {html}"
    );
    assert!(
        html.contains("WorkOrders"),
        "Requester nav missing WorkOrders: {html}"
    );
    assert!(
        !html.contains("Warehouse"),
        "Requester nav must not show Warehouse: {html}"
    );
    assert!(
        !html.contains("Admin"),
        "Requester nav must not show Admin: {html}"
    );
}

#[test]
fn intern_nav_has_internship_not_catalog_or_admin() {
    let html = render(nav_intern);
    assert!(
        html.contains("Internship"),
        "Intern nav missing Internship: {html}"
    );
    assert!(
        !html.contains("Catalog"),
        "Intern nav must not show Catalog: {html}"
    );
    assert!(
        !html.contains("Admin"),
        "Intern nav must not show Admin: {html}"
    );
    assert!(
        !html.contains("Warehouse"),
        "Intern nav must not show Warehouse: {html}"
    );
}

#[test]
fn warehouse_manager_nav_has_warehouse_not_catalog_or_admin() {
    let html = render(nav_warehouse_manager);
    assert!(
        html.contains("Warehouse"),
        "WarehouseManager nav missing Warehouse: {html}"
    );
    assert!(
        !html.contains("Catalog"),
        "WarehouseManager nav must not show Catalog: {html}"
    );
    assert!(
        !html.contains("Admin"),
        "WarehouseManager nav must not show Admin: {html}"
    );
}

#[test]
fn moderator_nav_has_forum_not_admin_or_warehouse() {
    let html = render(nav_moderator);
    assert!(
        html.contains("Forum"),
        "Moderator nav missing Forum: {html}"
    );
    assert!(
        !html.contains("Admin"),
        "Moderator nav must not show Admin: {html}"
    );
    assert!(
        !html.contains("Warehouse"),
        "Moderator nav must not show Warehouse: {html}"
    );
}

#[test]
fn mentor_nav_has_internship_not_catalog_or_warehouse() {
    let html = render(nav_mentor);
    assert!(
        html.contains("Internship"),
        "Mentor nav missing Internship: {html}"
    );
    assert!(
        !html.contains("Catalog"),
        "Mentor nav must not show Catalog: {html}"
    );
    assert!(
        !html.contains("Warehouse"),
        "Mentor nav must not show Warehouse: {html}"
    );
}

#[test]
fn service_manager_nav_has_catalog_and_work_orders_not_admin() {
    let html = render(nav_service_manager);
    assert!(
        html.contains("Catalog"),
        "ServiceManager nav missing Catalog: {html}"
    );
    assert!(
        html.contains("WorkOrders"),
        "ServiceManager nav missing WorkOrders: {html}"
    );
    assert!(
        !html.contains("Admin"),
        "ServiceManager nav must not show Admin: {html}"
    );
}

#[test]
fn home_is_always_first_rendered_item_for_every_role() {
    for (label, html) in [
        ("Administrator", render(nav_admin)),
        ("Requester", render(nav_requester)),
        ("Intern", render(nav_intern)),
        ("WarehouseManager", render(nav_warehouse_manager)),
        ("Moderator", render(nav_moderator)),
        ("Mentor", render(nav_mentor)),
        ("ServiceManager", render(nav_service_manager)),
    ] {
        let home_pos = html
            .find("Home")
            .expect(&format!("Home absent for {label}"));
        let other_module_pos = html
            .find("Catalog")
            .or_else(|| html.find("Warehouse"))
            .or_else(|| html.find("Internship"))
            .or_else(|| html.find("Admin"))
            .unwrap_or(usize::MAX);
        assert!(
            home_pos < other_module_pos,
            "Home must appear before other modules for {label}"
        );
    }
}

#[test]
fn face_appears_in_every_role_nav() {
    for (label, html) in [
        ("Administrator", render(nav_admin)),
        ("Requester", render(nav_requester)),
        ("Intern", render(nav_intern)),
        ("WarehouseManager", render(nav_warehouse_manager)),
        ("Moderator", render(nav_moderator)),
        ("Mentor", render(nav_mentor)),
        ("ServiceManager", render(nav_service_manager)),
    ] {
        assert!(
            html.contains("Face"),
            "Face must appear for {label}: {html}"
        );
    }
}

#[test]
fn forum_appears_in_every_role_nav() {
    for (label, html) in [
        ("Administrator", render(nav_admin)),
        ("Requester", render(nav_requester)),
        ("Intern", render(nav_intern)),
        ("WarehouseManager", render(nav_warehouse_manager)),
        ("Moderator", render(nav_moderator)),
        ("Mentor", render(nav_mentor)),
        ("ServiceManager", render(nav_service_manager)),
    ] {
        assert!(
            html.contains("Forum"),
            "Forum must appear for {label}: {html}"
        );
    }
}

// ---- pure-logic nav tests (no rendering) ----

#[test]
fn menu_for_admin_has_exactly_eight_items() {
    assert_eq!(
        menu_for(Role::Administrator).len(),
        8,
        "admin menu: Home Catalog WorkOrders Forum Internship Warehouse Face Admin"
    );
}

#[test]
fn menu_for_moderator_has_home_forum_face_only() {
    let m = menu_for(Role::Moderator);
    assert!(m.contains(&NavItem::Home));
    assert!(m.contains(&NavItem::Forum));
    assert!(m.contains(&NavItem::Face));
    assert!(!m.contains(&NavItem::Catalog));
    assert!(!m.contains(&NavItem::Warehouse));
    assert!(!m.contains(&NavItem::Admin));
    assert!(!m.contains(&NavItem::Internship));
    assert!(!m.contains(&NavItem::WorkOrders));
}
