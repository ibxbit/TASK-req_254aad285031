use dioxus::prelude::*;
use shared::Role;

use crate::auth::{self, use_auth, AuthState};
use crate::router::Route;

#[component]
pub fn AuthedLayout() -> Element {
    let auth_sig = use_auth();
    let state = auth_sig.0.read().clone();

    if !state.is_logged_in() {
        return rsx! {
            div { class: "unauth",
                p { "You are not signed in." }
                Link { to: Route::Login {}, "Go to sign in" }
            }
        };
    }

    let user = state.user.clone().unwrap();
    let role = user.role;
    let token = state.token.clone().unwrap_or_default();
    let mut sig = auth_sig.0;
    let nav = use_navigator();

    let logout = move |_| {
        let t = token.clone();
        spawn(async move {
            auth::logout(&t).await;
            sig.set(AuthState::default());
            nav.replace(Route::Login {});
        });
    };

    rsx! {
        style { {STYLE} }
        div { class: "app",
            header { class: "topbar",
                h1 { "Field Service Ops Hub" }
                div { class: "user",
                    span { "{user.username}" }
                    span { class: "role", " · {user.role.display_name()}" }
                    button { class: "signout", onclick: logout, "Sign out" }
                }
            }
            div { class: "body",
                NavBar { role: role }
                main { class: "content", Outlet::<Route> {} }
            }
        }
    }
}

#[component]
fn NavBar(role: Role) -> Element {
    rsx! {
        nav { class: "sidebar",
            Link { to: Route::Home {}, "Home" }
            if matches!(role, Role::Requester | Role::ServiceManager | Role::Administrator) {
                Link { to: Route::Catalog {}, "Catalog" }
                Link { to: Route::WorkOrders {}, "Work Orders" }
            }
            Link { to: Route::Forum {}, "Forum" }
            if matches!(role, Role::Intern | Role::Mentor | Role::Administrator) {
                Link { to: Route::Internship {}, "Internship" }
            }
            if matches!(role, Role::WarehouseManager | Role::Administrator) {
                Link { to: Route::Warehouse {}, "Warehouse" }
            }
            Link { to: Route::Face {}, "Face" }
            if role == Role::Administrator {
                Link { to: Route::Admin {}, "Admin" }
            }
        }
    }
}

const STYLE: &str = r#"
/* ---------- Base (desktop default, ≥1024px) ---------- */
body { margin: 0; font-family: system-ui, sans-serif; color: #222; background: #f5f6f8; }
.app { min-height: 100vh; display: flex; flex-direction: column; }
.topbar { display: flex; justify-content: space-between; align-items: center;
    padding: 0.6rem 1rem; background: #1e293b; color: #fff; flex-wrap: wrap; gap: 0.5rem; }
.topbar h1 { font-size: 1rem; margin: 0; }
.topbar .user { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.topbar .user .role { opacity: 0.7; font-size: 0.85rem; }
.topbar .signout { background: #475569; color: #fff; border: none;
    padding: 0.25rem 0.7rem; cursor: pointer; border-radius: 3px; }
.body { display: flex; flex: 1; }
.sidebar { width: 180px; background: #e2e8f0; padding: 1rem 0.5rem;
    display: flex; flex-direction: column; gap: 0.25rem; }
.sidebar a { color: #1e293b; text-decoration: none; padding: 0.4rem 0.6rem; border-radius: 3px; }
.sidebar a:hover { background: #cbd5e1; }
.content { flex: 1; padding: 1.5rem; background: #fff; min-width: 0; }
.content h2 { margin-top: 0; }
.card { border: 1px solid #e2e8f0; border-radius: 4px; padding: 0.75rem 1rem;
    margin-bottom: 0.5rem; background: #fafbfc; }
.row { display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.5rem; flex-wrap: wrap; }
.row label { min-width: 7rem; }
.row input, .row select, .row textarea { padding: 0.3rem 0.5rem; border: 1px solid #cbd5e1; border-radius: 3px; }
.err { color: #b91c1c; }
.muted { color: #64748b; font-size: 0.9rem; }
.login-page { max-width: 320px; margin: 3rem auto; background: #fff; padding: 2rem; border-radius: 4px; }
.login-page div { margin-bottom: 0.6rem; }
.login-page input { width: 100%; box-sizing: border-box; padding: 0.4rem; border: 1px solid #cbd5e1; }
.login-page button { padding: 0.5rem 1rem; background: #1e293b; color: #fff; border: none; cursor: pointer; }
.compare-grid { display: flex; gap: 1rem; align-items: stretch; flex-wrap: wrap; }
.compare-grid > * { flex: 1 1 260px; min-width: 0; }

/* ---------- Tablet (768–1023px) ---------- */
@media (min-width: 768px) and (max-width: 1023px) {
    .sidebar { width: 140px; padding: 0.75rem 0.4rem; }
    .content { padding: 1rem; }
    .topbar h1 { font-size: 0.95rem; }
    .row { gap: 0.4rem; }
    .row label { min-width: 5.5rem; font-size: 0.9rem; }
    .card { padding: 0.6rem 0.8rem; }
    .compare-grid > * { flex: 1 1 45%; }
}

/* ---------- Mobile / small tablet portrait (<768px) ---------- */
@media (max-width: 767px) {
    .body { flex-direction: column; }
    .sidebar { width: auto; flex-direction: row; flex-wrap: wrap;
        padding: 0.5rem; gap: 0.25rem; }
    .sidebar a { flex: 1 1 auto; text-align: center; font-size: 0.9rem; }
    .content { padding: 0.75rem; }
    .row { flex-direction: column; align-items: stretch; }
    .row label { min-width: 0; margin-bottom: 0.15rem; font-size: 0.85rem; }
    .row input, .row select, .row textarea { width: 100%; box-sizing: border-box; }
    .compare-grid { flex-direction: column; }
    .compare-grid > * { flex: 1 1 100%; }
}
"#;
