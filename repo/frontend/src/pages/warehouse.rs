// Warehouse management — mutation-capable UI for warehouse managers /
// administrators. Tree remains the default read view; mutations (create,
// rename, delete of warehouses / zones / bins) and change-history lookups
// are exposed in-page.

use dioxus::prelude::*;
use shared::{
    BinChangeLog, CreateBinRequest, Role, UpdateBinRequest, WarehouseChangeLog, WarehouseTreeNode,
    WarehouseZoneChangeLog,
};

use crate::api::warehouse;
use crate::auth::use_auth;

#[component]
pub fn Warehouse() -> Element {
    let role = use_auth()
        .0
        .read()
        .user
        .as_ref()
        .map(|u| u.role)
        .unwrap_or(Role::Requester);
    let can_mutate = matches!(role, Role::Administrator | Role::WarehouseManager);

    let mut tree = use_signal(Vec::<WarehouseTreeNode>::new);
    let mut error = use_signal(|| None::<String>);
    let mut info = use_signal(|| None::<String>);

    let reload = move || {
        spawn(async move {
            match warehouse::tree().await {
                Ok(t) => tree.set(t),
                Err(e) => error.set(Some(e)),
            }
        });
    };

    use_future(move || async move {
        match warehouse::tree().await {
            Ok(t) => tree.set(t),
            Err(e) => error.set(Some(e)),
        }
    });

    // Mutation form state.
    let mut new_wh_name = use_signal(String::new);
    let mut rename_wh_id = use_signal(String::new);
    let mut rename_wh_name = use_signal(String::new);
    let mut delete_wh_id = use_signal(String::new);

    let mut new_zone_wh = use_signal(String::new);
    let mut new_zone_name = use_signal(String::new);
    let mut delete_zone_id = use_signal(String::new);

    let mut new_bin = use_signal(|| CreateBinRequest {
        zone_id: String::new(),
        name: String::new(),
        width_in: 1.0,
        height_in: 1.0,
        depth_in: 1.0,
        max_load_lbs: 0.0,
        temp_zone: "ambient".into(),
        is_enabled: Some(true),
    });

    let mut update_bin_id = use_signal(String::new);
    let mut update_bin_enabled = use_signal(|| true);

    // History lookups.
    let mut hist_target = use_signal(|| "warehouse".to_string());
    let mut hist_id = use_signal(String::new);
    let mut hist_wh = use_signal(Vec::<WarehouseChangeLog>::new);
    let mut hist_zone = use_signal(Vec::<WarehouseZoneChangeLog>::new);
    let mut hist_bin = use_signal(Vec::<BinChangeLog>::new);

    let create_wh = move |_| {
        let n = new_wh_name();
        if n.trim().is_empty() {
            return;
        }
        spawn(async move {
            match warehouse::create_warehouse(n).await {
                Ok(_) => {
                    info.set(Some("Warehouse created".into()));
                    new_wh_name.set(String::new());
                    reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let rename_wh = move |_| {
        let id = rename_wh_id();
        let name = rename_wh_name();
        if id.trim().is_empty() || name.trim().is_empty() {
            return;
        }
        spawn(async move {
            match warehouse::rename_warehouse(&id, name).await {
                Ok(_) => {
                    info.set(Some("Warehouse renamed".into()));
                    reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let delete_wh = move |_| {
        let id = delete_wh_id();
        if id.trim().is_empty() {
            return;
        }
        spawn(async move {
            match warehouse::delete_warehouse(&id).await {
                Ok(_) => {
                    info.set(Some("Warehouse deleted".into()));
                    reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let create_zone = move |_| {
        let wid = new_zone_wh();
        let n = new_zone_name();
        if wid.trim().is_empty() || n.trim().is_empty() {
            return;
        }
        spawn(async move {
            match warehouse::create_zone(wid, n).await {
                Ok(_) => {
                    info.set(Some("Zone created".into()));
                    new_zone_name.set(String::new());
                    reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let delete_zone = move |_| {
        let id = delete_zone_id();
        if id.trim().is_empty() {
            return;
        }
        spawn(async move {
            match warehouse::delete_zone(&id).await {
                Ok(_) => {
                    info.set(Some("Zone deleted".into()));
                    reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let create_bin = move |_| {
        let req = new_bin();
        if req.zone_id.trim().is_empty() || req.name.trim().is_empty() {
            return;
        }
        spawn(async move {
            match warehouse::create_bin(req).await {
                Ok(_) => {
                    info.set(Some("Bin created".into()));
                    reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let toggle_bin = move |_| {
        let id = update_bin_id();
        let enabled = update_bin_enabled();
        if id.trim().is_empty() {
            return;
        }
        let patch = UpdateBinRequest {
            is_enabled: Some(enabled),
            ..Default::default()
        };
        spawn(async move {
            match warehouse::update_bin(&id, patch).await {
                Ok(_) => {
                    info.set(Some("Bin updated".into()));
                    reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let load_history = move |_| {
        let id = hist_id();
        let target = hist_target();
        if id.trim().is_empty() {
            return;
        }
        spawn(async move {
            match target.as_str() {
                "warehouse" => match warehouse::warehouse_history(&id).await {
                    Ok(v) => hist_wh.set(v),
                    Err(e) => error.set(Some(e)),
                },
                "zone" => match warehouse::zone_history(&id).await {
                    Ok(v) => hist_zone.set(v),
                    Err(e) => error.set(Some(e)),
                },
                _ => match warehouse::bin_history(&id).await {
                    Ok(v) => hist_bin.set(v),
                    Err(e) => error.set(Some(e)),
                },
            }
        });
    };

    rsx! {
        h2 { "Warehouse" }
        if let Some(m) = info() { p { class: "muted", "{m}" } }
        if let Some(m) = error() { p { class: "err", "{m}" } }

        // Tree (read)
        div { class: "card",
            h3 { "Tree" }
            if tree().is_empty() { p { class: "muted", "No warehouses." } }
            for wh in tree().into_iter() {
                div { key: "{wh.id}",
                    strong { "{wh.name}" }
                    span { class: "muted", " · {wh.id}" }
                    for z in wh.zones.into_iter() {
                        div { key: "{z.id}", style: "margin-left: 1rem;",
                            strong { "Zone {z.name}" }
                            span { class: "muted", " · {z.id}" }
                            for b in z.bins.into_iter() {
                                div { key: "{b.id}", style: "margin-left: 1rem;",
                                    "Bin {b.name} — {b.width_in}x{b.height_in}x{b.depth_in} in, "
                                    "{b.max_load_lbs} lbs, {b.temp_zone}, "
                                    if b.is_enabled { span { "enabled" } } else { span { class: "muted", "disabled" } }
                                    span { class: "muted", " · {b.id}" }
                                }
                            }
                        }
                    }
                }
            }
        }

        if !can_mutate {
            p { class: "muted",
                "You need the warehouse_manager or administrator role to mutate structure."
            }
        }

        // Mutations — backend also enforces the role guard; we hide the
        // controls to avoid presenting a UI that will always fail.
        if can_mutate {
        div { class: "card",
            h3 { "Warehouses" }
            div { class: "row",
                input { placeholder: "New name", value: "{new_wh_name}",
                    oninput: move |e| new_wh_name.set(e.value()) }
                button { onclick: create_wh, "Create warehouse" }
            }
            div { class: "row",
                input { placeholder: "Id", value: "{rename_wh_id}",
                    oninput: move |e| rename_wh_id.set(e.value()) }
                input { placeholder: "New name", value: "{rename_wh_name}",
                    oninput: move |e| rename_wh_name.set(e.value()) }
                button { onclick: rename_wh, "Rename" }
            }
            div { class: "row",
                input { placeholder: "Id to delete", value: "{delete_wh_id}",
                    oninput: move |e| delete_wh_id.set(e.value()) }
                button { onclick: delete_wh, "Delete" }
            }
        }

        div { class: "card",
            h3 { "Zones" }
            div { class: "row",
                input { placeholder: "Warehouse id", value: "{new_zone_wh}",
                    oninput: move |e| new_zone_wh.set(e.value()) }
                input { placeholder: "Zone name", value: "{new_zone_name}",
                    oninput: move |e| new_zone_name.set(e.value()) }
                button { onclick: create_zone, "Create zone" }
            }
            div { class: "row",
                input { placeholder: "Zone id to delete", value: "{delete_zone_id}",
                    oninput: move |e| delete_zone_id.set(e.value()) }
                button { onclick: delete_zone, "Delete" }
            }
        }

        div { class: "card",
            h3 { "Bins" }
            div { class: "row",
                input { placeholder: "Zone id",
                    value: "{new_bin().zone_id}",
                    oninput: move |e| { let mut v = new_bin(); v.zone_id = e.value(); new_bin.set(v); } }
                input { placeholder: "Name",
                    value: "{new_bin().name}",
                    oninput: move |e| { let mut v = new_bin(); v.name = e.value(); new_bin.set(v); } }
                input { placeholder: "Temp zone",
                    value: "{new_bin().temp_zone}",
                    oninput: move |e| { let mut v = new_bin(); v.temp_zone = e.value(); new_bin.set(v); } }
                button { onclick: create_bin, "Create bin" }
            }
            div { class: "row",
                input { placeholder: "Bin id", value: "{update_bin_id}",
                    oninput: move |e| update_bin_id.set(e.value()) }
                label { "Enabled" }
                input { r#type: "checkbox", checked: "{update_bin_enabled}",
                    onchange: move |e| update_bin_enabled.set(e.value() == "true") }
                button { onclick: toggle_bin, "Apply" }
            }
        }

        }

        div { class: "card",
            h3 { "Change history" }
            div { class: "row",
                label { "Kind" }
                select {
                    value: "{hist_target}",
                    onchange: move |e| hist_target.set(e.value()),
                    option { value: "warehouse", "Warehouse" }
                    option { value: "zone", "Zone" }
                    option { value: "bin", "Bin" }
                }
                input { placeholder: "Entity id", value: "{hist_id}",
                    oninput: move |e| hist_id.set(e.value()) }
                button { onclick: load_history, "Load" }
            }
            if hist_target() == "warehouse" {
                for h in hist_wh().into_iter() {
                    div { key: "{h.id}", class: "muted",
                        "{h.created_at} · {h.change_type} · by {h.changed_by}"
                    }
                }
            } else if hist_target() == "zone" {
                for h in hist_zone().into_iter() {
                    div { key: "{h.id}", class: "muted",
                        "{h.created_at} · {h.change_type} · by {h.changed_by}"
                    }
                }
            } else {
                for h in hist_bin().into_iter() {
                    div { key: "{h.id}", class: "muted",
                        "{h.created_at} · {h.change_type} · by {h.changed_by}"
                    }
                }
            }
        }
    }
}
