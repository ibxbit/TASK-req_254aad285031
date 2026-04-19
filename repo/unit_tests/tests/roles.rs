use shared::Role;

#[test]
fn all_roles_round_trip_through_str() {
    let all = [
        Role::Administrator,
        Role::Moderator,
        Role::ServiceManager,
        Role::WarehouseManager,
        Role::Mentor,
        Role::Intern,
        Role::Requester,
    ];
    for r in all {
        let s = r.as_str();
        assert_eq!(Role::from_str(s), Some(r), "round-trip for {s}");
    }
}

#[test]
fn unknown_role_string_is_rejected() {
    assert_eq!(Role::from_str("emperor"), None);
    assert_eq!(Role::from_str(""), None);
    assert_eq!(
        Role::from_str("Administrator"),
        None,
        "match is case-sensitive"
    );
}

#[test]
fn role_serde_json_uses_snake_case() {
    let v = serde_json::to_string(&Role::ServiceManager).unwrap();
    assert_eq!(v, "\"service_manager\"");
    let r: Role = serde_json::from_str("\"warehouse_manager\"").unwrap();
    assert_eq!(r, Role::WarehouseManager);
}

#[test]
fn role_display_name_is_human_readable() {
    assert_eq!(Role::ServiceManager.display_name(), "Service Manager");
    assert_eq!(Role::Intern.display_name(), "Intern");
}
