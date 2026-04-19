use dioxus::prelude::*;
use shared::Role;

use crate::auth::use_auth;
use crate::router::Route;

#[component]
pub fn Home() -> Element {
    let state = use_auth().0.read().clone();
    let user = match state.user.clone() {
        Some(u) => u,
        None => {
            return rsx! {
                div { class: "unauth",
                    p { "Not signed in." }
                    Link { to: Route::Login {}, "Sign in" }
                }
            };
        }
    };

    let role = user.role;

    rsx! {
        h2 { "Welcome, {user.username}" }
        p { class: "muted", "Role: {role.display_name()}" }

        div { class: "card",
            h3 { "Modules available to you" }
            ul {
                if matches!(role, Role::Requester | Role::ServiceManager | Role::Administrator) {
                    li { Link { to: Route::Catalog {}, "Service catalog & search" } }
                    li { Link { to: Route::WorkOrders {}, "Work orders & reviews" } }
                }
                li { Link { to: Route::Forum {}, "Forum" } }
                if matches!(role, Role::Intern | Role::Mentor | Role::Administrator) {
                    li { Link { to: Route::Internship {}, "Internship dashboard" } }
                }
                if matches!(role, Role::WarehouseManager | Role::Administrator) {
                    li { Link { to: Route::Warehouse {}, "Warehouse" } }
                }
                li { Link { to: Route::Face {}, "Face records" } }
            }
        }
    }
}
