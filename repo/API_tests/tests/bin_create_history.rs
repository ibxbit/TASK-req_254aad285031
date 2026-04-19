//! Bin creation must appear in bin_change_log (history endpoint), paralleling
//! warehouse + zone history which both record the 'create' event.

use api_tests::{assert_status, get_auth, json_body, nano_suffix, post_json_auth, setup};
use serde_json::json;

#[test]
fn creating_a_bin_appears_in_its_history() {
    let Some(admin) = setup("creating_a_bin_appears_in_its_history") else {
        return;
    };

    // Build a fresh warehouse -> zone -> bin chain.
    let wh = post_json_auth(
        "/api/warehouses",
        &admin,
        &json!({"name": format!("wh_{}", nano_suffix())}),
    )
    .expect("wh");
    assert_status(&wh, 200, "create warehouse");
    let wid = json_body(wh, "wh")["id"].as_str().unwrap().to_string();

    let zone = post_json_auth(
        "/api/warehouse-zones",
        &admin,
        &json!({"warehouse_id": wid, "name": format!("z_{}", nano_suffix())}),
    )
    .expect("z");
    assert_status(&zone, 200, "create zone");
    let zid = json_body(zone, "z")["id"].as_str().unwrap().to_string();

    let bin_name = format!("b_{}", nano_suffix());
    let bin = post_json_auth(
        "/api/bins",
        &admin,
        &json!({
            "zone_id": zid,
            "name": bin_name,
            "width_in": 12.0,
            "height_in": 10.0,
            "depth_in": 8.0,
            "max_load_lbs": 50.0,
            "temp_zone": "ambient",
        }),
    )
    .expect("bin");
    assert_status(&bin, 200, "create bin");
    let bid = json_body(bin, "bin")["id"].as_str().unwrap().to_string();

    // ---- The fix under test: history for this bin must contain a row
    // of change_type = "create" right after the insert. ----
    let hist = get_auth(&format!("/api/bins/{bid}/history"), &admin).expect("h");
    assert_status(&hist, 200, "GET /api/bins/<id>/history");
    let v = json_body(hist, "h");
    let arr = v.as_array().expect("array body");
    assert!(
        !arr.is_empty(),
        "bin history must record the create event, got empty"
    );
    let has_create = arr
        .iter()
        .any(|r| r["change_type"].as_str() == Some("create"));
    assert!(
        has_create,
        "bin history must contain a 'create' row, got: {v}"
    );

    // Contract shape for each entry.
    let first = &arr[0];
    api_tests::assert_keys(
        first,
        &["id", "bin_id", "changed_by", "change_type", "created_at"],
        "BinChangeLog",
    );
    // changed_by must be the admin who created the bin (not null / blank).
    let changed_by = first["changed_by"].as_str().unwrap_or("");
    assert_eq!(
        changed_by.len(),
        36,
        "changed_by must be a UUID, got `{changed_by}`"
    );
}
