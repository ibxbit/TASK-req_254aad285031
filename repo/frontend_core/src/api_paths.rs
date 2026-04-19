//! Centralised API path builders. Having one source of truth for each
//! endpoint means a typo is caught by tests instead of by a runtime 404.

pub fn services_list() -> &'static str {
    "/api/services/search"
}

pub fn service_by_id(id: &str) -> String {
    format!("/api/services/{id}")
}

pub fn service_reviews(id: &str) -> String {
    format!("/api/services/{id}/reviews")
}

pub fn service_reputation(id: &str) -> String {
    format!("/api/services/{id}/reputation")
}

pub fn service_compare(ids: &[String]) -> String {
    format!("/api/services/compare?ids={}", ids.join(","))
}

pub fn work_order_by_id(id: &str) -> String {
    format!("/api/work-orders/{id}")
}

pub fn work_order_complete(id: &str) -> String {
    format!("/api/work-orders/{id}/complete")
}

pub fn work_order_follow_up(id: &str) -> String {
    format!("/api/work-orders/{id}/follow-up-review")
}

pub fn review_images(id: &str) -> String {
    format!("/api/reviews/{id}/images")
}

pub fn review_tag_assign(id: &str) -> String {
    format!("/api/reviews/{id}/tags")
}

pub fn board_posts(id: &str) -> String {
    format!("/api/boards/{id}/posts")
}

pub fn board_rules(id: &str) -> String {
    format!("/api/boards/{id}/rules")
}

pub fn board_moderators(id: &str) -> String {
    format!("/api/boards/{id}/moderators")
}

pub fn board_moderator(id: &str, user_id: &str) -> String {
    format!("/api/boards/{id}/moderators/{user_id}")
}

pub fn board_teams(id: &str) -> String {
    format!("/api/boards/{id}/teams")
}

pub fn board_team(id: &str, team_id: &str) -> String {
    format!("/api/boards/{id}/teams/{team_id}")
}

pub fn warehouse_history(id: &str) -> String {
    format!("/api/warehouses/{id}/history")
}

pub fn zone_history(id: &str) -> String {
    format!("/api/warehouse-zones/{id}/history")
}

pub fn bin_history(id: &str) -> String {
    format!("/api/bins/{id}/history")
}

pub fn intern_dashboard(id: &str) -> String {
    format!("/api/interns/{id}/dashboard")
}

pub fn report_comments(id: &str) -> String {
    format!("/api/reports/{id}/comments")
}

pub fn report_approve(id: &str) -> String {
    format!("/api/reports/{id}/approve")
}

pub fn report_attachments(id: &str) -> String {
    format!("/api/reports/{id}/attachments")
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests are the contract that the backend routes in
    // backend/src/routes/*.rs accept. If either side changes one the other
    // must follow — this test fails first.
    #[test]
    fn service_paths_match_backend_routes() {
        assert_eq!(service_by_id("abc"), "/api/services/abc");
        assert_eq!(service_reviews("abc"), "/api/services/abc/reviews");
        assert_eq!(service_reputation("abc"), "/api/services/abc/reputation");
        assert_eq!(
            service_compare(&["a".into(), "b".into()]),
            "/api/services/compare?ids=a,b"
        );
    }

    #[test]
    fn work_order_paths_match_backend_routes() {
        assert_eq!(work_order_by_id("w"), "/api/work-orders/w");
        assert_eq!(work_order_complete("w"), "/api/work-orders/w/complete");
        assert_eq!(
            work_order_follow_up("w"),
            "/api/work-orders/w/follow-up-review"
        );
    }

    #[test]
    fn review_paths_match_backend_routes() {
        assert_eq!(review_images("r"), "/api/reviews/r/images");
        assert_eq!(review_tag_assign("r"), "/api/reviews/r/tags");
    }

    #[test]
    fn warehouse_history_paths_include_bins() {
        // Regression guard: the `/api/bins/<id>/history` endpoint is auth
        // protected (parity with warehouse + zone history). The frontend
        // must hit this exact path.
        assert_eq!(warehouse_history("w"), "/api/warehouses/w/history");
        assert_eq!(zone_history("z"), "/api/warehouse-zones/z/history");
        assert_eq!(bin_history("b"), "/api/bins/b/history");
    }

    #[test]
    fn intern_and_report_paths_match_backend_routes() {
        assert_eq!(intern_dashboard("u"), "/api/interns/u/dashboard");
        assert_eq!(report_comments("r"), "/api/reports/r/comments");
        assert_eq!(report_approve("r"), "/api/reports/r/approve");
        assert_eq!(report_attachments("r"), "/api/reports/r/attachments");
    }

    #[test]
    fn board_paths_match_backend_routes() {
        assert_eq!(board_posts("b"), "/api/boards/b/posts");
        assert_eq!(board_rules("b"), "/api/boards/b/rules");
        assert_eq!(board_moderators("b"), "/api/boards/b/moderators");
        assert_eq!(board_moderator("b", "u"), "/api/boards/b/moderators/u");
        assert_eq!(board_teams("b"), "/api/boards/b/teams");
        assert_eq!(board_team("b", "t"), "/api/boards/b/teams/t");
    }
}
