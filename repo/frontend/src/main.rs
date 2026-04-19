use dioxus::prelude::*;

mod api;
mod auth;
mod components;
mod pages;
mod router;

use router::Route;

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| auth::AuthSignal(Signal::new(auth::load_auth_state())));
    rsx! { Router::<Route> {} }
}
