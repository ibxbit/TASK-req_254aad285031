use shared::{
    BinChangeLog, CreateBinRequest, CreateWarehouseRequest, CreateWarehouseZoneRequest,
    UpdateBinRequest, Warehouse, WarehouseChangeLog, WarehouseTreeNode, WarehouseZone,
    WarehouseZoneChangeLog,
};

use super::client;

pub async fn tree() -> Result<Vec<WarehouseTreeNode>, String> {
    client::get_json("/api/warehouses/tree").await
}

pub async fn create_warehouse(name: String) -> Result<Warehouse, String> {
    let body = CreateWarehouseRequest { name };
    client::post_json("/api/warehouses", &body).await
}

pub async fn rename_warehouse(id: &str, name: String) -> Result<(), String> {
    let body = CreateWarehouseRequest { name };
    client::patch_json(&format!("/api/warehouses/{id}"), &body).await
}

pub async fn delete_warehouse(id: &str) -> Result<(), String> {
    client::delete_empty(&format!("/api/warehouses/{id}")).await
}

pub async fn create_zone(warehouse_id: String, name: String) -> Result<WarehouseZone, String> {
    let body = CreateWarehouseZoneRequest { warehouse_id, name };
    client::post_json("/api/warehouse-zones", &body).await
}

pub async fn rename_zone(id: &str, name: String) -> Result<(), String> {
    client::patch_json(
        &format!("/api/warehouse-zones/{id}"),
        &serde_json::json!({ "name": name }),
    )
    .await
}

pub async fn delete_zone(id: &str) -> Result<(), String> {
    client::delete_empty(&format!("/api/warehouse-zones/{id}")).await
}

pub async fn create_bin(req: CreateBinRequest) -> Result<shared::Bin, String> {
    client::post_json("/api/bins", &req).await
}

pub async fn update_bin(id: &str, patch: UpdateBinRequest) -> Result<(), String> {
    client::patch_json(&format!("/api/bins/{id}"), &patch).await
}

pub async fn warehouse_history(id: &str) -> Result<Vec<WarehouseChangeLog>, String> {
    client::get_json(&format!("/api/warehouses/{id}/history")).await
}

pub async fn zone_history(id: &str) -> Result<Vec<WarehouseZoneChangeLog>, String> {
    client::get_json(&format!("/api/warehouse-zones/{id}/history")).await
}

pub async fn bin_history(id: &str) -> Result<Vec<BinChangeLog>, String> {
    client::get_json(&format!("/api/bins/{id}/history")).await
}
