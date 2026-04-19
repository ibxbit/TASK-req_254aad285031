//! Contract coverage for /api/warehouses, /api/warehouse-zones, /api/bins,
//! including history endpoints and structural mutation paths.

use api_tests::{
    assert_keys, assert_status, delete_auth, get_auth, json_body, login, nano_suffix,
    patch_json_auth, post_json_auth, provision_user, setup,
};
use serde_json::{json, Value};

fn create_warehouse(admin: &str, name_prefix: &str) -> Option<String> {
    let r = post_json_auth(
        "/api/warehouses",
        admin,
        &json!({"name": format!("{name_prefix}_{}", nano_suffix())}),
    )
    .ok()?;
    if !r.status().is_success() {
        return None;
    }
    let v: Value = r.json().ok()?;
    Some(v["id"].as_str()?.to_string())
}

#[test]
fn warehouses_crud_contract_and_history() {
    let Some(admin) = setup("warehouses_crud_contract_and_history") else {
        return;
    };

    // Create -> 200 + contract.
    let name = format!("wh_{}", nano_suffix());
    let create = post_json_auth("/api/warehouses", &admin, &json!({"name": name})).expect("c");
    assert_status(&create, 200, "POST /warehouses");
    let v = json_body(create, "wh");
    assert_keys(&v, &["id", "name"], "Warehouse");
    assert_eq!(v["name"], name);
    let wid = v["id"].as_str().unwrap().to_string();

    // Duplicate name -> 409.
    let dup = post_json_auth("/api/warehouses", &admin, &json!({"name": name})).expect("dup");
    assert_status(&dup, 409, "duplicate warehouse name");

    // Rename (idempotent with same name -> 204 no-op).
    let rename_same = patch_json_auth(
        &format!("/api/warehouses/{wid}"),
        &admin,
        &json!({"name": name}),
    )
    .expect("r1");
    assert_status(&rename_same, 204, "rename same-name no-op");

    let new_name = format!("wh2_{}", nano_suffix());
    let rename_ok = patch_json_auth(
        &format!("/api/warehouses/{wid}"),
        &admin,
        &json!({"name": new_name.clone()}),
    )
    .expect("r2");
    assert_status(&rename_ok, 204, "rename ok");

    // Empty name -> 400.
    let rename_bad = patch_json_auth(
        &format!("/api/warehouses/{wid}"),
        &admin,
        &json!({"name":""}),
    )
    .expect("rb");
    assert_status(&rename_bad, 400, "empty name");

    // Rename missing -> 404.
    let rename_missing = patch_json_auth(
        "/api/warehouses/00000000-0000-0000-0000-000000000000",
        &admin,
        &json!({"name":"x"}),
    )
    .expect("rm");
    assert_status(&rename_missing, 404, "rename missing");

    // Tree.
    let tree = get_auth("/api/warehouses/tree", &admin).expect("t");
    assert_status(&tree, 200, "GET /warehouses/tree");
    let tv = json_body(tree, "t");
    assert!(tv.is_array());
    let found = tv
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["id"].as_str() == Some(&wid))
        .expect("tree must include created warehouse");
    assert_keys(found, &["id", "name", "zones"], "TreeNode");

    // History -> 200 array with >=1 entry (create + rename).
    let hist = get_auth(&format!("/api/warehouses/{wid}/history"), &admin).expect("h");
    assert_status(&hist, 200, "GET history");
    let hv = json_body(hist, "h");
    let arr = hv.as_array().unwrap();
    assert!(!arr.is_empty(), "history should record create + rename");
    assert_keys(
        &arr[0],
        &[
            "id",
            "warehouse_id",
            "changed_by",
            "change_type",
            "created_at",
        ],
        "WarehouseChangeLog",
    );

    // Delete -> 204.
    let del = delete_auth(&format!("/api/warehouses/{wid}"), &admin).expect("d");
    assert_status(&del, 204, "DELETE /warehouses/<id>");
    let del_again = delete_auth(&format!("/api/warehouses/{wid}"), &admin).expect("d2");
    assert_status(&del_again, 404, "DELETE missing warehouse");
}

#[test]
fn zones_and_bins_crud_contract() {
    let Some(admin) = setup("zones_and_bins_crud_contract") else {
        return;
    };
    let Some(wid) = create_warehouse(&admin, "wh") else {
        return;
    };

    // Zone create.
    let zname = format!("z_{}", nano_suffix());
    let zc = post_json_auth(
        "/api/warehouse-zones",
        &admin,
        &json!({"warehouse_id": wid, "name": zname}),
    )
    .expect("zc");
    assert_status(&zc, 200, "POST /warehouse-zones");
    let zv = json_body(zc, "zc");
    assert_keys(&zv, &["id", "warehouse_id", "name"], "WarehouseZone");
    let zid = zv["id"].as_str().unwrap().to_string();

    // Zone rename.
    let new_z = format!("z2_{}", nano_suffix());
    let zr = patch_json_auth(
        &format!("/api/warehouse-zones/{zid}"),
        &admin,
        &json!({"name": new_z}),
    )
    .expect("zr");
    assert_status(&zr, 204, "rename zone");

    // Zone history.
    let zh = get_auth(&format!("/api/warehouse-zones/{zid}/history"), &admin).expect("zh");
    assert_status(&zh, 200, "zone history");
    let hv = json_body(zh, "zh");
    assert!(hv.is_array());
    assert_keys(
        &hv.as_array().unwrap()[0],
        &["id", "zone_id", "changed_by", "change_type", "created_at"],
        "ZoneChangeLog",
    );

    // Bin create with invalid dimensions -> 400.
    let bad_bin = post_json_auth(
        "/api/bins",
        &admin,
        &json!({
            "zone_id": zid,
            "name": "bad",
            "width_in": 0.0,
            "height_in": 1.0,
            "depth_in": 1.0,
            "max_load_lbs": 0.0,
            "temp_zone":"ambient"
        }),
    )
    .expect("bad");
    assert_status(&bad_bin, 400, "bin zero width");

    // Bin create valid.
    let bin_body = json!({
        "zone_id": zid,
        "name": format!("b_{}", nano_suffix()),
        "width_in": 1.0,
        "height_in": 2.0,
        "depth_in": 3.0,
        "max_load_lbs": 10.0,
        "temp_zone": "ambient"
    });
    let bc = post_json_auth("/api/bins", &admin, &bin_body).expect("bc");
    assert_status(&bc, 200, "POST /bins");
    let bv = json_body(bc, "bc");
    assert_keys(
        &bv,
        &[
            "id",
            "zone_id",
            "name",
            "width_in",
            "height_in",
            "depth_in",
            "max_load_lbs",
            "temp_zone",
            "is_enabled",
        ],
        "Bin",
    );
    let bid = bv["id"].as_str().unwrap().to_string();

    // Patch bin (toggle disabled).
    let bu = patch_json_auth(
        &format!("/api/bins/{bid}"),
        &admin,
        &json!({"is_enabled": false}),
    )
    .expect("bu");
    assert_status(&bu, 204, "PATCH /bins/<id>");

    // Patch missing bin -> 404.
    let missing = patch_json_auth(
        "/api/bins/00000000-0000-0000-0000-000000000000",
        &admin,
        &json!({"is_enabled": false}),
    )
    .expect("miss");
    assert_status(&missing, 404, "PATCH missing bin");

    // Bin history.
    let bh = get_auth(&format!("/api/bins/{bid}/history"), &admin).expect("bh");
    assert_status(&bh, 200, "GET /bins/<id>/history");
    let bhv = json_body(bh, "bh");
    assert!(bhv.is_array());
    assert!(
        !bhv.as_array().unwrap().is_empty(),
        "bin history records update"
    );
    assert_keys(
        &bhv.as_array().unwrap()[0],
        &["id", "bin_id", "changed_by", "change_type", "created_at"],
        "BinChangeLog",
    );

    // Delete zone.
    let zd = delete_auth(&format!("/api/warehouse-zones/{zid}"), &admin).expect("zd");
    assert_status(&zd, 204, "DELETE zone");
    let zd2 = delete_auth(&format!("/api/warehouse-zones/{zid}"), &admin).expect("zd2");
    assert_status(&zd2, 404, "DELETE missing zone");
}

#[test]
fn warehouse_management_role_matrix() {
    let Some(admin) = setup("warehouse_management_role_matrix") else {
        return;
    };
    let Some((_, u, p)) = provision_user(&admin, "warehouse_manager") else {
        return;
    };
    let Some(wm) = login(&u, &p) else {
        return;
    };

    // Warehouse manager can read tree + create warehouse.
    let t = get_auth("/api/warehouses/tree", &wm).expect("t");
    assert_status(&t, 200, "wm tree");

    let c = post_json_auth(
        "/api/warehouses",
        &wm,
        &json!({"name": format!("wm_{}", nano_suffix())}),
    )
    .expect("c");
    assert_status(&c, 200, "wm create warehouse");

    // Other non-management roles: 403.
    for role in [
        "requester",
        "moderator",
        "intern",
        "mentor",
        "service_manager",
    ]
    .iter()
    {
        let Some((_, u, p)) = provision_user(&admin, role) else {
            continue;
        };
        let Some(tok) = login(&u, &p) else {
            continue;
        };
        let r = post_json_auth(
            "/api/warehouses",
            &tok,
            &json!({"name": format!("no_{}", nano_suffix())}),
        )
        .expect("r");
        assert_status(&r, 403, &format!("{role} must not create warehouses"));
    }
}
