//! JSON round-trips for DTOs that cross the HTTP boundary. Catches serde
//! attribute regressions (rename, skip_serializing_if, default) before they
//! break the frontend.

use shared::{
    CreateReviewRequest, CreateServiceRequest, EventLog, LoginRequest, Reputation,
    ReputationBreakdownEntry, Service, SortMode,
};

#[test]
fn login_request_round_trip() {
    let req = LoginRequest {
        username: "alice".into(),
        password: "correct horse battery staple".into(),
    };
    let s = serde_json::to_string(&req).unwrap();
    let back: LoginRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.username, req.username);
    assert_eq!(back.password, req.password);
}

#[test]
fn create_review_request_has_no_followup_field() {
    // After the one-review-per-order tightening there is no is_followup.
    let j = r#"{"work_order_id":"11111111-1111-1111-1111-111111111111","rating":5,"text":"great"}"#;
    let r: CreateReviewRequest = serde_json::from_str(j).unwrap();
    assert_eq!(r.rating, 5);
    assert_eq!(r.text, "great");

    // Extra field is ignored by serde (forward compatible); the field itself
    // must not exist on the struct.
    let any = serde_json::to_value(&r).unwrap();
    assert!(
        any.get("is_followup").is_none(),
        "is_followup removed from DTO"
    );
}

#[test]
fn service_json_shape() {
    let svc = Service {
        id: "s1".into(),
        name: "Pipe Repair".into(),
        description: "fast".into(),
        price: 99.5,
        rating: 4.2,
        coverage_radius_miles: 25,
        zip_code: "94110".into(),
    };
    let v = serde_json::to_value(&svc).unwrap();
    assert_eq!(v["name"], "Pipe Repair");
    assert_eq!(v["price"], 99.5);
    assert_eq!(v["coverage_radius_miles"], 25);
    let back: Service = serde_json::from_value(v).unwrap();
    assert_eq!(back.zip_code, "94110");
}

#[test]
fn create_service_request_rating_defaults_to_none() {
    let j = r#"{"name":"x","description":"y","price":10.0,"coverage_radius_miles":5,"zip_code":"12345"}"#;
    let r: CreateServiceRequest = serde_json::from_str(j).unwrap();
    assert!(r.rating.is_none());
}

#[test]
fn sort_mode_json_names_match_spec() {
    assert_eq!(SortMode::from_str("best_rated"), Some(SortMode::BestRated));
    assert_eq!(
        SortMode::from_str("lowest_price"),
        Some(SortMode::LowestPrice)
    );
    assert_eq!(
        SortMode::from_str("soonest_available"),
        Some(SortMode::SoonestAvailable)
    );
    assert!(SortMode::from_str("SOONEST_AVAILABLE").is_none());
}

#[test]
fn reputation_breakdown_is_optional_but_roundtrips() {
    // With breakdown = None the `breakdown` key is omitted (skip_serializing_if).
    let r = Reputation {
        service_id: "abc".into(),
        final_score: 4.5,
        total_reviews: 10,
        breakdown: None,
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["final_score"], 4.5);
    assert_eq!(v["total_reviews"], 10);
    assert!(
        v.get("breakdown").is_none(),
        "None breakdown should be omitted"
    );

    // With Some(...), breakdown is serialized.
    let entry = ReputationBreakdownEntry {
        review_id: "r1".into(),
        rating: 5,
        days_since: 3.5,
        weight: 0.98,
        created_at: chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap(),
    };
    let r2 = Reputation {
        service_id: "abc".into(),
        final_score: 4.5,
        total_reviews: 1,
        breakdown: Some(vec![entry]),
    };
    let v2 = serde_json::to_value(&r2).unwrap();
    assert!(v2["breakdown"].is_array());
    assert_eq!(v2["breakdown"][0]["rating"], 5);
}

#[test]
fn event_log_fields_present() {
    // A shape test — an event_log JSON from the backend must deserialize into
    // the shared EventLog DTO without losing chain fields (prev_hash/hash).
    let j = serde_json::json!({
        "id": "e1",
        "sequence": 42,
        "entity_type": "review",
        "entity_id": "r1",
        "action": "create",
        "payload": "{\"action\":\"create\"}",
        "prev_hash": "0".repeat(64),
        "hash": "a".repeat(64),
        "created_at": "2026-04-18T12:00:00",
    });
    let e: EventLog = serde_json::from_value(j).unwrap();
    assert_eq!(e.sequence, 42);
    assert_eq!(e.prev_hash.len(), 64);
    assert_eq!(e.hash.len(), 64);
}
