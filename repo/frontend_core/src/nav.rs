//! Navigation visibility: given a role, which top-level modules should
//! the user see? The layout + home components delegate to these so the
//! same rule lives in one place and is testable without a browser.

use shared::Role;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NavItem {
    Home,
    Catalog,
    WorkOrders,
    Forum,
    Internship,
    Warehouse,
    Face,
    Admin,
}

/// Modules available to the given role, in stable display order.
pub fn menu_for(role: Role) -> Vec<NavItem> {
    let mut out = vec![NavItem::Home];
    if matches!(
        role,
        Role::Requester | Role::ServiceManager | Role::Administrator
    ) {
        out.push(NavItem::Catalog);
        out.push(NavItem::WorkOrders);
    }
    out.push(NavItem::Forum);
    if matches!(role, Role::Intern | Role::Mentor | Role::Administrator) {
        out.push(NavItem::Internship);
    }
    if matches!(role, Role::WarehouseManager | Role::Administrator) {
        out.push(NavItem::Warehouse);
    }
    out.push(NavItem::Face);
    if role == Role::Administrator {
        out.push(NavItem::Admin);
    }
    out
}

/// Every authenticated user can reach Home, Forum, Face. Everything else
/// depends on role.
pub fn role_can_see(role: Role, item: &NavItem) -> bool {
    menu_for(role).contains(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_sees_everything_including_admin_panel() {
        let m = menu_for(Role::Administrator);
        for item in [
            NavItem::Home,
            NavItem::Catalog,
            NavItem::WorkOrders,
            NavItem::Forum,
            NavItem::Internship,
            NavItem::Warehouse,
            NavItem::Face,
            NavItem::Admin,
        ] {
            assert!(m.contains(&item), "admin should see {item:?}");
        }
    }

    #[test]
    fn requester_sees_catalog_and_work_orders_not_warehouse() {
        let m = menu_for(Role::Requester);
        assert!(m.contains(&NavItem::Catalog));
        assert!(m.contains(&NavItem::WorkOrders));
        assert!(!m.contains(&NavItem::Warehouse));
        assert!(!m.contains(&NavItem::Admin));
    }

    #[test]
    fn warehouse_manager_sees_warehouse_not_catalog() {
        let m = menu_for(Role::WarehouseManager);
        assert!(m.contains(&NavItem::Warehouse));
        assert!(!m.contains(&NavItem::Catalog));
        assert!(!m.contains(&NavItem::Admin));
    }

    #[test]
    fn intern_sees_internship_not_admin_or_warehouse() {
        let m = menu_for(Role::Intern);
        assert!(m.contains(&NavItem::Internship));
        assert!(!m.contains(&NavItem::Admin));
        assert!(!m.contains(&NavItem::Warehouse));
    }

    #[test]
    fn mentor_sees_internship_but_not_catalog_or_warehouse() {
        let m = menu_for(Role::Mentor);
        assert!(m.contains(&NavItem::Internship));
        assert!(!m.contains(&NavItem::Catalog));
        assert!(!m.contains(&NavItem::Warehouse));
    }

    #[test]
    fn moderator_sees_forum_not_admin() {
        let m = menu_for(Role::Moderator);
        assert!(m.contains(&NavItem::Forum));
        assert!(!m.contains(&NavItem::Admin));
    }

    #[test]
    fn role_can_see_agrees_with_menu_for() {
        assert!(role_can_see(Role::Administrator, &NavItem::Admin));
        assert!(!role_can_see(Role::Requester, &NavItem::Admin));
        assert!(role_can_see(Role::Requester, &NavItem::WorkOrders));
    }

    #[test]
    fn menu_always_starts_with_home() {
        for role in [
            Role::Administrator,
            Role::Moderator,
            Role::ServiceManager,
            Role::WarehouseManager,
            Role::Mentor,
            Role::Intern,
            Role::Requester,
        ] {
            assert_eq!(menu_for(role)[0], NavItem::Home, "role {role:?} Home first");
        }
    }
}
