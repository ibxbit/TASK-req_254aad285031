use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warehouse {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWarehouseRequest {
    pub name: String,
}

// Named WarehouseZone to avoid collision with forum::Zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseZone {
    pub id: String,
    pub warehouse_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWarehouseZoneRequest {
    pub warehouse_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bin {
    pub id: String,
    pub zone_id: String,
    pub name: String,
    pub width_in: f64,
    pub height_in: f64,
    pub depth_in: f64,
    pub max_load_lbs: f64,
    pub temp_zone: String,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBinRequest {
    pub zone_id: String,
    pub name: String,
    pub width_in: f64,
    pub height_in: f64,
    pub depth_in: f64,
    pub max_load_lbs: f64,
    pub temp_zone: String,
    pub is_enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateBinRequest {
    pub name: Option<String>,
    pub width_in: Option<f64>,
    pub height_in: Option<f64>,
    pub depth_in: Option<f64>,
    pub max_load_lbs: Option<f64>,
    pub temp_zone: Option<String>,
    pub is_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinChangeLog {
    pub id: String,
    pub bin_id: String,
    pub changed_by: String,
    pub change_type: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseChangeLog {
    pub id: String,
    pub warehouse_id: String,
    pub changed_by: String,
    pub change_type: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseZoneChangeLog {
    pub id: String,
    pub zone_id: String,
    pub changed_by: String,
    pub change_type: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub created_at: NaiveDateTime,
}

// --- Tree response ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseZoneNode {
    pub id: String,
    pub name: String,
    pub bins: Vec<Bin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseTreeNode {
    pub id: String,
    pub name: String,
    pub zones: Vec<WarehouseZoneNode>,
}
