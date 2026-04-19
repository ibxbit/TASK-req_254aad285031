// Structured traceability writes for the warehouse hierarchy.
//
// Covers warehouse and warehouse-zone mutations (rename, create, delete).
// Bin changes have their own dedicated helper in routes/warehouse/bins.rs;
// the tamper-evident `event_log` hash chain covers all three levels.

use sqlx::{MySql, Transaction};
use uuid::Uuid;

pub async fn log_warehouse_change(
    tx: &mut Transaction<'_, MySql>,
    warehouse_id: Uuid,
    changed_by: Uuid,
    change_type: &str,
    old: Option<String>,
    new: Option<String>,
) -> sqlx::Result<()> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouse_change_log \
         (id, warehouse_id, changed_by, change_type, old_value, new_value) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id.as_bytes()[..])
    .bind(&warehouse_id.as_bytes()[..])
    .bind(&changed_by.as_bytes()[..])
    .bind(change_type)
    .bind(old)
    .bind(new)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn log_zone_change(
    tx: &mut Transaction<'_, MySql>,
    zone_id: Uuid,
    changed_by: Uuid,
    change_type: &str,
    old: Option<String>,
    new: Option<String>,
) -> sqlx::Result<()> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouse_zone_change_log \
         (id, zone_id, changed_by, change_type, old_value, new_value) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id.as_bytes()[..])
    .bind(&zone_id.as_bytes()[..])
    .bind(&changed_by.as_bytes()[..])
    .bind(change_type)
    .bind(old)
    .bind(new)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
