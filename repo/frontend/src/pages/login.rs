use dioxus::prelude::*;

use crate::auth::{self, save_auth_state, use_auth, AuthState};
use crate::router::Route;

#[component]
pub fn Login() -> Element {
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let error = use_signal(|| None::<String>);
    let auth_sig = use_auth();
    let nav = use_navigator();

    let submit = move |_| {
        let u = username();
        let p = password();
        let mut sig = auth_sig.0;
        let mut err = error;
        spawn(async move {
            match auth::login(u, p).await {
                Ok(resp) => {
                    let state = AuthState {
                        token: Some(resp.token),
                        user: Some(resp.user),
                    };
                    save_auth_state(&state);
                    sig.set(state);
                    nav.replace(Route::Home {});
                }
                Err(e) => err.set(Some(e)),
            }
        });
    };

    rsx! {
        div { class: "login-page",
            h1 { "Sign in" }
            div {
                label { "Username" }
                input {
                    r#type: "text",
                    value: "{username}",
                    oninput: move |e| username.set(e.value()),
                }
            }
            div {
                label { "Password" }
                input {
                    r#type: "password",
                    value: "{password}",
                    oninput: move |e| password.set(e.value()),
                }
            }
            button { onclick: submit, "Sign in" }
            if let Some(msg) = error() {
                p { class: "error", "{msg}" }
            }
        }
    }
}
