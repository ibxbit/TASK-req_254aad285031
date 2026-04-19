//! Route paths used by the Dioxus router. Keeping them as constants lets
//! the frontend and tests agree on the URL surface without importing
//! Dioxus macros.

pub const LOGIN: &str = "/login";
pub const HOME: &str = "/";
pub const CATALOG: &str = "/catalog";
pub const WORK_ORDERS: &str = "/work-orders";
pub const FORUM: &str = "/forum";
pub const INTERNSHIP: &str = "/internship";
pub const WAREHOUSE: &str = "/warehouse";
pub const FACE: &str = "/face";
pub const ADMIN: &str = "/admin";

/// When the user is unauthenticated and hits a protected route, the guard
/// redirects to `/login` but remembers where they came from. This helper
/// returns the post-login destination for a given prior path (returns
/// `None` when the user was already at /login or at an unauth surface).
pub fn post_login_redirect(from: &str) -> Option<&'static str> {
    match from {
        LOGIN => None,
        HOME => Some(HOME),
        CATALOG => Some(CATALOG),
        WORK_ORDERS => Some(WORK_ORDERS),
        FORUM => Some(FORUM),
        INTERNSHIP => Some(INTERNSHIP),
        WAREHOUSE => Some(WAREHOUSE),
        FACE => Some(FACE),
        ADMIN => Some(ADMIN),
        // Unknown route — fall back to Home rather than 404 after sign-in.
        _ => Some(HOME),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_paths_are_stable() {
        // Pinned so a rename can't silently break external bookmarks.
        assert_eq!(LOGIN, "/login");
        assert_eq!(HOME, "/");
        assert_eq!(CATALOG, "/catalog");
        assert_eq!(WORK_ORDERS, "/work-orders");
        assert_eq!(FORUM, "/forum");
        assert_eq!(INTERNSHIP, "/internship");
        assert_eq!(WAREHOUSE, "/warehouse");
        assert_eq!(FACE, "/face");
        assert_eq!(ADMIN, "/admin");
    }

    #[test]
    fn login_is_not_a_post_login_destination() {
        assert_eq!(post_login_redirect(LOGIN), None);
    }

    #[test]
    fn known_routes_roundtrip() {
        assert_eq!(post_login_redirect(FORUM), Some(FORUM));
        assert_eq!(post_login_redirect(ADMIN), Some(ADMIN));
        assert_eq!(post_login_redirect(WORK_ORDERS), Some(WORK_ORDERS));
    }

    #[test]
    fn unknown_route_falls_back_to_home() {
        assert_eq!(post_login_redirect("/nope"), Some(HOME));
    }
}
