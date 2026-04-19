use shared::{ReportStatus, ReportType, VisibilityType, WorkOrderStatus};

#[test]
fn work_order_status_round_trip() {
    for s in [
        WorkOrderStatus::Pending,
        WorkOrderStatus::InProgress,
        WorkOrderStatus::Completed,
        WorkOrderStatus::Cancelled,
    ] {
        assert_eq!(WorkOrderStatus::from_str(s.as_str()), Some(s));
    }
    assert_eq!(WorkOrderStatus::from_str("exploded"), None);
}

#[test]
fn report_type_is_uppercase_on_the_wire() {
    let v = serde_json::to_string(&ReportType::Weekly).unwrap();
    assert_eq!(v, "\"WEEKLY\"");
    let r: ReportType = serde_json::from_str("\"DAILY\"").unwrap();
    assert_eq!(r, ReportType::Daily);
}

#[test]
fn report_status_is_snake_case() {
    assert_eq!(ReportStatus::OnTime.as_str(), "on_time");
    assert_eq!(ReportStatus::Late.as_str(), "late");
    assert_eq!(
        ReportStatus::from_str("on_time"),
        Some(ReportStatus::OnTime)
    );
    assert_eq!(ReportStatus::from_str("ontime"), None);
}

#[test]
fn visibility_type_is_snake_case() {
    assert_eq!(VisibilityType::Public.as_str(), "public");
    assert_eq!(VisibilityType::Restricted.as_str(), "restricted");
    assert_eq!(
        VisibilityType::from_str("restricted"),
        Some(VisibilityType::Restricted)
    );
    assert!(VisibilityType::from_str("PUBLIC").is_none());
}
