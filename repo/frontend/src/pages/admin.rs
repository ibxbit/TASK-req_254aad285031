use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use shared::Role;

use crate::api::client::{get_json, post_json};
use crate::auth::use_auth;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AdminUser {
    pub id: String,
    pub username: String,
    pub role: Role,
    pub is_active: bool,
    pub sensitive_id_mask: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateUserReq {
    pub username: String,
    pub password: String,
    pub role: Role,
}

#[component]
pub fn Admin() -> Element {
    let auth = use_auth();
    let role = auth
        .0
        .read()
        .user
        .as_ref()
        .map(|u| u.role)
        .unwrap_or(Role::Requester);

    if role != Role::Administrator {
        return rsx! {
            h2 { "Admin" }
            p { class: "err", "You do not have permission to view this page." }
        };
    }

    let mut users = use_signal(Vec::<AdminUser>::new);
    let mut error = use_signal(|| None::<String>);

    use_future(move || async move {
        match get_json::<Vec<AdminUser>>("/api/admin/users").await {
            Ok(list) => users.set(list),
            Err(e) => error.set(Some(e)),
        }
    });

    let mut new_username = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut new_role = use_signal(|| Role::Requester);

    let create_user = move |_| {
        let u = new_username();
        let p = new_password();
        let r = new_role();
        spawn(async move {
            let req = CreateUserReq {
                username: u,
                password: p,
                role: r,
            };
            match post_json::<_, AdminUser>("/api/admin/users", &req).await {
                Ok(new_user) => {
                    users.with_mut(|list| list.push(new_user));
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    rsx! {
        h2 { "Administration" }
        if let Some(e) = error() { p { class: "err", "{e}" } }

        div { class: "card",
            h3 { "Provision user" }
            div { class: "row",
                label { "Username" }
                input {
                    r#type: "text",
                    value: "{new_username}",
                    oninput: move |e| new_username.set(e.value()),
                }
                pre { hidden: true, "{new_username}" }
            }
            div { class: "row",
                label { "Password" }
                input {
                    r#type: "password",
                    value: "{new_password}",
                    oninput: move |e| new_password.set(e.value()),
                }
            }
            div { class: "row",
                label { "Role" }
                select {
                    value: "{new_role().as_str()}",
                    onchange: move |e| {
                        if let Some(r) = Role::from_str(&e.value()) {
                            new_role.set(r);
                        }
                    },
                    option { value: "administrator",     "Administrator" }
                    option { value: "moderator",         "Moderator" }
                    option { value: "service_manager",   "Service Manager" }
                    option { value: "warehouse_manager", "Warehouse Manager" }
                    option { value: "mentor",            "Mentor" }
                    option { value: "intern",            "Intern" }
                    option { value: "requester",         "Requester" }
                }
            }
            button { onclick: create_user, "Create user" }
        }

        div { class: "card",
            h3 { "Users" }
            if users().is_empty() { p { class: "muted", "No users yet." } }
            for u in users().into_iter() {
                div { key: "{u.id}", class: "row",
                    span { "{u.username}" }
                    span { class: "role", " · {u.role.display_name()}" }
                    if !u.is_active { span { class: "muted", " (inactive)" } }
                    if let Some(m) = u.sensitive_id_mask.clone() {
                        span { class: "muted", " · id: {m}" }
                    }
                }
            }
        }
    }
}
