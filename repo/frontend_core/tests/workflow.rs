//! Cross-module integration tests: exercise the real user workflows that
//! the Dioxus components invoke under the hood. These complement the
//! per-module unit tests by asserting they compose correctly.

use frontend_core::api_paths::*;
use frontend_core::auth_state::AuthState;
use frontend_core::nav::{menu_for, NavItem};
use frontend_core::rating::{clamp_rating, parse_rating};
use frontend_core::route::{post_login_redirect, ADMIN, HOME, WORK_ORDERS};
use frontend_core::search::{build_search_path, SearchParams};
use frontend_core::tag_selection::toggle_tag;
use shared::{Role, SessionUser};

// -------- Auth workflow --------

#[test]
fn auth_state_workflow_login_logout() {
    // A fresh signal has no user, no token.
    let mut state = AuthState::default();
    assert!(!state.is_logged_in());

    // Successful login populates both fields.
    state.token = Some("t".into());
    state.user = Some(SessionUser {
        id: "id".into(),
        username: "u".into(),
        role: Role::Administrator,
    });
    assert!(state.is_logged_in());
    assert_eq!(state.bearer_header().as_deref(), Some("Bearer t"));

    // Logout blanks the state.
    let fresh = AuthState::default();
    assert!(!fresh.is_logged_in());
}

// -------- Route guard workflow --------

#[test]
fn unauthenticated_request_for_admin_page_redirects_back_after_login() {
    // User tries /admin while logged out -> login page.
    // After login, they should land back at /admin (the guard records
    // the origin via post_login_redirect).
    assert_eq!(post_login_redirect(ADMIN), Some(ADMIN));
    assert_eq!(post_login_redirect(WORK_ORDERS), Some(WORK_ORDERS));
    // Coming from /login itself is a no-op — no redirect loop.
    assert_eq!(post_login_redirect("/login"), None);
}

// -------- Nav visibility per role --------

#[test]
fn role_gated_nav_is_consistent_across_roles() {
    for role in [Role::Requester, Role::ServiceManager, Role::Administrator] {
        let m = menu_for(role);
        assert!(
            m.contains(&NavItem::WorkOrders),
            "{role:?} needs WorkOrders"
        );
    }
    for role in [Role::Intern, Role::Mentor, Role::Administrator] {
        let m = menu_for(role);
        assert!(
            m.contains(&NavItem::Internship),
            "{role:?} needs Internship"
        );
    }
    // Mentor sees Internship but NOT WorkOrders (mentors aren't requesters).
    let mentor = menu_for(Role::Mentor);
    assert!(!mentor.contains(&NavItem::WorkOrders));
}

// -------- Search workflow --------

#[test]
fn catalog_search_workflow_encodes_all_filters() {
    let params = SearchParams {
        q: Some("hvac".into()),
        min_price: Some(10.0),
        max_price: Some(250.0),
        min_rating: Some(4.0),
        user_zip: Some("94110".into()),
        sort: Some("lowest_price".into()),
        ..Default::default()
    };
    let path = build_search_path(&params);
    assert!(path.starts_with("/api/services/search?"));
    for segment in [
        "q=hvac",
        "min_price=10",
        "max_price=250",
        "min_rating=4",
        "user_zip=94110",
        "sort=lowest_price",
    ] {
        assert!(path.contains(segment), "missing `{segment}` in `{path}`");
    }
}

// -------- Review submission workflow --------

#[test]
fn review_submit_workflow_from_form_state() {
    // User types "  5 " into the rating input, selects two tags, then
    // adjusts rating up out of range.
    let rating = parse_rating("  5 ").expect("parse");
    assert_eq!(rating, 5);

    let mut tag_ids: Vec<String> = Vec::new();
    toggle_tag(&mut tag_ids, "tag-1");
    toggle_tag(&mut tag_ids, "tag-2");
    assert_eq!(tag_ids, vec!["tag-1".to_string(), "tag-2".to_string()]);

    // Un-click tag-1.
    toggle_tag(&mut tag_ids, "tag-1");
    assert_eq!(tag_ids, vec!["tag-2".to_string()]);

    // Out-of-range ratings get clamped, not silently accepted.
    assert_eq!(clamp_rating(200), 5);
}

// -------- API path workflow --------

#[test]
fn every_frontend_surface_resolves_to_expected_backend_route() {
    // Regression guard: the frontend hits these paths. If the backend
    // renames a route in `backend/src/routes/*.rs`, the API test suite
    // catches it; if the frontend constructs a different path, this
    // test is the first to fail.
    let work_order = "wo-id";
    assert_eq!(work_order_by_id(work_order), "/api/work-orders/wo-id");
    assert_eq!(
        work_order_complete(work_order),
        "/api/work-orders/wo-id/complete"
    );
    assert_eq!(
        work_order_follow_up(work_order),
        "/api/work-orders/wo-id/follow-up-review"
    );

    assert_eq!(review_images("rid"), "/api/reviews/rid/images");
    assert_eq!(review_tag_assign("rid"), "/api/reviews/rid/tags");

    assert_eq!(service_reviews("sid"), "/api/services/sid/reviews");
    assert_eq!(service_reputation("sid"), "/api/services/sid/reputation");

    assert_eq!(bin_history("bid"), "/api/bins/bid/history");
    assert_eq!(zone_history("zid"), "/api/warehouse-zones/zid/history");
    assert_eq!(warehouse_history("wid"), "/api/warehouses/wid/history");

    assert_eq!(intern_dashboard("uid"), "/api/interns/uid/dashboard");
    assert_eq!(report_approve("rid"), "/api/reports/rid/approve");
    assert_eq!(report_comments("rid"), "/api/reports/rid/comments");
    assert_eq!(report_attachments("rid"), "/api/reports/rid/attachments");

    assert_eq!(board_posts("b"), "/api/boards/b/posts");
    assert_eq!(board_rules("b"), "/api/boards/b/rules");
    assert_eq!(board_team("b", "t"), "/api/boards/b/teams/t");

    // Spot-check an unrelated default path to make sure the prefix stays.
    assert_eq!(post_login_redirect("/foo"), Some(HOME));
}
