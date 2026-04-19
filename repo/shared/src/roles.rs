use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Administrator,
    Moderator,
    ServiceManager,
    WarehouseManager,
    Mentor,
    Intern,
    Requester,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Administrator => "administrator",
            Role::Moderator => "moderator",
            Role::ServiceManager => "service_manager",
            Role::WarehouseManager => "warehouse_manager",
            Role::Mentor => "mentor",
            Role::Intern => "intern",
            Role::Requester => "requester",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "administrator" => Some(Role::Administrator),
            "moderator" => Some(Role::Moderator),
            "service_manager" => Some(Role::ServiceManager),
            "warehouse_manager" => Some(Role::WarehouseManager),
            "mentor" => Some(Role::Mentor),
            "intern" => Some(Role::Intern),
            "requester" => Some(Role::Requester),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Role::Administrator => "Administrator",
            Role::Moderator => "Moderator",
            Role::ServiceManager => "Service Manager",
            Role::WarehouseManager => "Warehouse Manager",
            Role::Mentor => "Mentor",
            Role::Intern => "Intern",
            Role::Requester => "Requester",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
