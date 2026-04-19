// Forum page — one surface for everyone but with role-gated admin controls
// folded in. Everyone can browse boards and post. Administrators additionally
// get zone create/delete, board create/update/delete, board rules, moderator
// assignment, and allowed-team management for RESTRICTED board visibility.

use dioxus::prelude::*;
use shared::{Board, BoardRule, Post, Role, VisibilityType, Zone};

use crate::api::forum;
use crate::auth::use_auth;

#[component]
pub fn Forum() -> Element {
    let role = use_auth()
        .0
        .read()
        .user
        .as_ref()
        .map(|u| u.role)
        .unwrap_or(Role::Requester);
    let is_admin = role == Role::Administrator;

    let mut boards = use_signal(Vec::<Board>::new);
    let mut zones = use_signal(Vec::<Zone>::new);
    let mut selected = use_signal(|| None::<String>);
    let mut posts = use_signal(Vec::<Post>::new);
    let mut rules = use_signal(Vec::<BoardRule>::new);
    let mut new_title = use_signal(String::new);
    let mut new_body = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut info = use_signal(|| None::<String>);

    // Admin-only form state — hoisted so the same buttons work across re-renders.
    let mut new_zone_name = use_signal(String::new);
    let mut new_board_zone = use_signal(String::new);
    let mut new_board_name = use_signal(String::new);
    let mut new_board_visibility = use_signal(|| "public".to_string());
    let mut new_rule = use_signal(String::new);
    let mut new_moderator_user = use_signal(String::new);
    let mut new_team_id = use_signal(String::new);

    let refresh = move || {
        spawn(async move {
            match forum::list_boards().await {
                Ok(list) => boards.set(list),
                Err(e) => error.set(Some(e)),
            }
            if is_admin {
                if let Ok(z) = forum::list_zones().await {
                    zones.set(z);
                }
            }
        });
    };

    use_future(move || async move {
        if let Ok(list) = forum::list_boards().await {
            boards.set(list);
        }
        if is_admin {
            if let Ok(z) = forum::list_zones().await {
                zones.set(z);
            }
        }
    });

    let open = move |board_id: String| {
        spawn(async move {
            match forum::list_posts(&board_id).await {
                Ok(list) => {
                    posts.set(list);
                    selected.set(Some(board_id.clone()));
                    if let Ok(r) = forum::list_board_rules(&board_id).await {
                        rules.set(r);
                    }
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let submit_post = move |_| {
        let Some(bid) = selected() else {
            return;
        };
        let title = new_title();
        let body = new_body();
        if title.trim().is_empty() || body.trim().is_empty() {
            return;
        }
        spawn(async move {
            match forum::create_post(bid.clone(), title, body).await {
                Ok(_) => {
                    new_title.set(String::new());
                    new_body.set(String::new());
                    if let Ok(list) = forum::list_posts(&bid).await {
                        posts.set(list);
                    }
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    // --- admin actions ---
    let create_zone = move |_| {
        let name = new_zone_name();
        if name.trim().is_empty() {
            return;
        }
        spawn(async move {
            match forum::create_zone(name).await {
                Ok(_) => {
                    new_zone_name.set(String::new());
                    info.set(Some("Zone created".into()));
                    refresh();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let delete_zone = move |id: String| {
        spawn(async move {
            match forum::delete_zone(&id).await {
                Ok(_) => {
                    info.set(Some("Zone deleted".into()));
                    refresh();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let create_board = move |_| {
        let zid = new_board_zone();
        let name = new_board_name();
        if zid.trim().is_empty() || name.trim().is_empty() {
            return;
        }
        let vis = match new_board_visibility().as_str() {
            "restricted" => VisibilityType::Restricted,
            _ => VisibilityType::Public,
        };
        spawn(async move {
            match forum::create_board(zid, name, vis).await {
                Ok(_) => {
                    new_board_name.set(String::new());
                    info.set(Some("Board created".into()));
                    refresh();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let delete_board = move |id: String| {
        spawn(async move {
            match forum::delete_board(&id).await {
                Ok(_) => {
                    info.set(Some("Board deleted".into()));
                    refresh();
                    selected.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let toggle_visibility = move |b: Board| {
        let new_vis = if b.visibility_type == VisibilityType::Public {
            VisibilityType::Restricted
        } else {
            VisibilityType::Public
        };
        spawn(async move {
            match forum::update_board(&b.id, None, Some(new_vis)).await {
                Ok(_) => {
                    info.set(Some("Visibility updated".into()));
                    refresh();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let add_rule = move |_| {
        let Some(bid) = selected() else {
            return;
        };
        let text = new_rule();
        if text.trim().is_empty() {
            return;
        }
        spawn(async move {
            match forum::create_board_rule(&bid, text).await {
                Ok(_) => {
                    new_rule.set(String::new());
                    if let Ok(r) = forum::list_board_rules(&bid).await {
                        rules.set(r);
                    }
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let delete_rule = move |rid: String| {
        let Some(bid) = selected() else {
            return;
        };
        spawn(async move {
            match forum::delete_board_rule(&rid).await {
                Ok(_) => {
                    if let Ok(r) = forum::list_board_rules(&bid).await {
                        rules.set(r);
                    }
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let add_mod = move |_| {
        let Some(bid) = selected() else {
            return;
        };
        let uid = new_moderator_user();
        if uid.trim().is_empty() {
            return;
        }
        spawn(async move {
            match forum::add_moderator(&bid, uid).await {
                Ok(_) => {
                    new_moderator_user.set(String::new());
                    info.set(Some("Moderator added".into()));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let allow_team = move |_| {
        let Some(bid) = selected() else {
            return;
        };
        let tid = new_team_id();
        if tid.trim().is_empty() {
            return;
        }
        spawn(async move {
            match forum::allow_team(&bid, tid).await {
                Ok(_) => {
                    new_team_id.set(String::new());
                    info.set(Some("Team granted access".into()));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    rsx! {
        h2 { "Forum" }
        if let Some(msg) = info() { p { class: "muted", "{msg}" } }
        if let Some(msg) = error() { p { class: "err", "{msg}" } }

        div { class: "row",
            // Boards list column.
            div { style: "min-width: 240px;",
                h3 { "Boards" }
                if boards().is_empty() {
                    p { class: "muted", "No accessible boards." }
                }
                for b in boards().into_iter() {
                    div { key: "{b.id}", class: "card",
                        button {
                            onclick: {
                                let id = b.id.clone();
                                move |_| open(id.clone())
                            },
                            "{b.name}"
                        }
                        span { class: "muted", " · {b.visibility_type:?}" }
                        if is_admin {
                            div { class: "row",
                                button {
                                    onclick: {
                                        let bc = b.clone();
                                        move |_| toggle_visibility(bc.clone())
                                    },
                                    "Toggle visibility"
                                }
                                button {
                                    onclick: {
                                        let id = b.id.clone();
                                        move |_| delete_board(id.clone())
                                    },
                                    "Delete board"
                                }
                            }
                        }
                    }
                }

                // Admin-only zone/board creation.
                if is_admin {
                    div { class: "card",
                        h4 { "Create zone" }
                        div { class: "row",
                            input {
                                placeholder: "zone name",
                                value: "{new_zone_name}",
                                oninput: move |e| new_zone_name.set(e.value()),
                            }
                            button { onclick: create_zone, "Create" }
                        }
                        if !zones().is_empty() {
                            p { "Zones:" }
                            for z in zones().into_iter() {
                                div { key: "{z.id}", class: "row",
                                    span { "{z.name}" }
                                    button {
                                        onclick: {
                                            let id = z.id.clone();
                                            move |_| delete_zone(id.clone())
                                        },
                                        "Delete"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "card",
                        h4 { "Create board" }
                        div { class: "row",
                            label { "Zone id" }
                            input {
                                value: "{new_board_zone}",
                                oninput: move |e| new_board_zone.set(e.value()),
                            }
                        }
                        div { class: "row",
                            label { "Name" }
                            input {
                                value: "{new_board_name}",
                                oninput: move |e| new_board_name.set(e.value()),
                            }
                        }
                        div { class: "row",
                            label { "Visibility" }
                            select {
                                value: "{new_board_visibility}",
                                onchange: move |e| new_board_visibility.set(e.value()),
                                option { value: "public", "Public" }
                                option { value: "restricted", "Restricted (team-gated)" }
                            }
                        }
                        button { onclick: create_board, "Create board" }
                    }
                }
            }

            // Selected board column.
            div { style: "flex: 1;",
                h3 { "New post" }
                div { class: "row",
                    input {
                        r#type: "text",
                        placeholder: "Title",
                        value: "{new_title}",
                        oninput: move |e| new_title.set(e.value()),
                    }
                }
                div { class: "row",
                    textarea {
                        placeholder: "Content",
                        rows: "4",
                        value: "{new_body}",
                        oninput: move |e| new_body.set(e.value()),
                    }
                }
                button { onclick: submit_post, "Post" }
                if let Some(bid) = selected() {
                    h3 { "Posts" }
                    for p in posts().into_iter() {
                        div { key: "{p.id}", class: "card",
                            if p.is_pinned { strong { "[pinned] " } }
                            strong { "{p.title}" }
                            p { "{p.content}" }
                        }
                    }

                    // Board rules — visible to anyone who can read the board.
                    h3 { "Rules" }
                    if rules().is_empty() {
                        p { class: "muted", "No rules." }
                    }
                    for r in rules().into_iter() {
                        div { key: "{r.id}", class: "card",
                            span { "{r.content}" }
                            if is_admin {
                                button {
                                    onclick: {
                                        let id = r.id.clone();
                                        move |_| delete_rule(id.clone())
                                    },
                                    "Delete"
                                }
                            }
                        }
                    }

                    if is_admin {
                        div { class: "card",
                            h4 { "Add rule" }
                            div { class: "row",
                                input {
                                    value: "{new_rule}",
                                    oninput: move |e| new_rule.set(e.value()),
                                }
                                button { onclick: add_rule, "Add" }
                            }
                        }
                        div { class: "card",
                            h4 { "Assign moderator" }
                            div { class: "row",
                                label { "User id" }
                                input {
                                    value: "{new_moderator_user}",
                                    oninput: move |e| new_moderator_user.set(e.value()),
                                }
                                button { onclick: add_mod, "Assign" }
                            }
                        }
                        div { class: "card",
                            h4 { "Grant team access" }
                            p { class: "muted",
                                "Only meaningful for Restricted boards — members of these teams can see the board."
                            }
                            div { class: "row",
                                label { "Team id" }
                                input {
                                    value: "{new_team_id}",
                                    oninput: move |e| new_team_id.set(e.value()),
                                }
                                button { onclick: allow_team, "Grant" }
                            }
                        }
                    }

                    span { class: "muted", " board: {bid}" }
                } else {
                    p { class: "muted", "Select a board to view posts." }
                }
            }
        }
    }
}
