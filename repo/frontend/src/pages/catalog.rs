use dioxus::prelude::*;
use shared::{Category, Service, ServiceComparison, Tag};

use crate::api::catalog::{self, SearchParams};

const COMPARE_LIMIT: usize = 3;

#[component]
pub fn Catalog() -> Element {
    // Text / numeric filters
    let mut q = use_signal(String::new);
    let mut min_price = use_signal(String::new);
    let mut max_price = use_signal(String::new);
    let mut min_rating = use_signal(String::new);
    let mut user_zip = use_signal(String::new);
    let mut sort = use_signal(|| "best_rated".to_string());

    // Availability window (ISO 8601 local-datetime, matches backend parser
    // `%Y-%m-%dT%H:%M:%S`).
    let mut available_from = use_signal(String::new);
    let mut available_to = use_signal(String::new);

    // Category + tag filters. Loaded once on mount from the browse GETs.
    let mut category_options = use_signal(Vec::<Category>::new);
    let mut tag_options = use_signal(Vec::<Tag>::new);
    let mut selected_categories = use_signal(Vec::<String>::new);
    let mut selected_tags = use_signal(Vec::<String>::new);

    let mut results = use_signal(Vec::<Service>::new);
    let mut error = use_signal(|| None::<String>);

    // Compare selection state.
    let mut selected = use_signal(Vec::<String>::new);
    let mut comparison = use_signal(|| None::<Vec<ServiceComparison>>);
    let mut compare_error = use_signal(|| None::<String>);

    use_future(move || async move {
        if let Ok(cs) = catalog::list_categories().await {
            category_options.set(cs);
        }
        if let Ok(ts) = catalog::list_tags().await {
            tag_options.set(ts);
        }
    });

    let do_search = move |_| {
        let params = SearchParams {
            q: Some(q()),
            min_price: min_price().trim().parse().ok(),
            max_price: max_price().trim().parse().ok(),
            min_rating: min_rating().trim().parse().ok(),
            user_zip: Some(user_zip()),
            sort: Some(sort()),
            available_from: Some(available_from()),
            available_to: Some(available_to()),
            categories: selected_categories(),
            tags: selected_tags(),
            limit: None,
            offset: None,
        };
        spawn(async move {
            match catalog::search(&params).await {
                Ok(list) => {
                    error.set(None);
                    results.set(list);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let fav = move |id: String| {
        spawn(async move {
            let _ = catalog::favorite(&id).await;
        });
    };

    let mut toggle_compare = move |id: String| {
        let mut sel = selected.write();
        if let Some(pos) = sel.iter().position(|s| s == &id) {
            sel.remove(pos);
        } else if sel.len() < COMPARE_LIMIT {
            sel.push(id);
        }
    };

    let run_compare = move |_| {
        let ids = selected();
        spawn(async move {
            match catalog::compare(&ids).await {
                Ok(list) => {
                    compare_error.set(None);
                    comparison.set(Some(list));
                }
                Err(e) => {
                    comparison.set(None);
                    compare_error.set(Some(e));
                }
            }
        });
    };

    let clear_compare = move |_| {
        selected.set(Vec::new());
        comparison.set(None);
        compare_error.set(None);
    };

    let mut toggle_category = move |id: String| {
        let mut v = selected_categories.write();
        if let Some(pos) = v.iter().position(|x| x == &id) {
            v.remove(pos);
        } else {
            v.push(id);
        }
    };
    let mut toggle_tag = move |id: String| {
        let mut v = selected_tags.write();
        if let Some(pos) = v.iter().position(|x| x == &id) {
            v.remove(pos);
        } else {
            v.push(id);
        }
    };

    rsx! {
        h2 { "Service Catalog" }
        div { class: "card",
            div { class: "row",
                label { "Search" }
                input { value: "{q}", oninput: move |e| q.set(e.value()) }
            }
            div { class: "row",
                label { "Min price" }
                input { value: "{min_price}", oninput: move |e| min_price.set(e.value()) }
                label { "Max price" }
                input { value: "{max_price}", oninput: move |e| max_price.set(e.value()) }
            }
            div { class: "row",
                label { "Min rating" }
                input { value: "{min_rating}", oninput: move |e| min_rating.set(e.value()) }
                label { "ZIP" }
                input { value: "{user_zip}", oninput: move |e| user_zip.set(e.value()) }
            }
            div { class: "row",
                label { "Available from" }
                input {
                    r#type: "datetime-local",
                    value: "{available_from}",
                    oninput: move |e| available_from.set(e.value()),
                }
                label { "to" }
                input {
                    r#type: "datetime-local",
                    value: "{available_to}",
                    oninput: move |e| available_to.set(e.value()),
                }
            }

            // Category filter: checkbox list built from GET /api/categories.
            if !category_options().is_empty() {
                p { class: "muted", "Categories (match ALL)" }
                div { class: "row",
                    for c in category_options().into_iter() {
                        label { key: "{c.id}",
                            input {
                                r#type: "checkbox",
                                checked: selected_categories().contains(&c.id),
                                onchange: {
                                    let id = c.id.clone();
                                    move |_| toggle_category(id.clone())
                                },
                            }
                            " {c.name}"
                        }
                    }
                }
            }

            // Tag filter: checkbox list built from GET /api/tags.
            if !tag_options().is_empty() {
                p { class: "muted", "Tags (match ANY)" }
                div { class: "row",
                    for t in tag_options().into_iter() {
                        label { key: "{t.id}",
                            input {
                                r#type: "checkbox",
                                checked: selected_tags().contains(&t.id),
                                onchange: {
                                    let id = t.id.clone();
                                    move |_| toggle_tag(id.clone())
                                },
                            }
                            " {t.name}"
                        }
                    }
                }
            }

            div { class: "row",
                label { "Sort" }
                select {
                    value: "{sort}",
                    onchange: move |e| sort.set(e.value()),
                    option { value: "best_rated", "Best rated" }
                    option { value: "lowest_price", "Lowest price" }
                    option { value: "soonest_available", "Soonest available" }
                }
                button { onclick: do_search, "Search" }
            }
            if let Some(msg) = error() {
                p { class: "err", "{msg}" }
            }
        }

        // Compare action bar
        div { class: "card",
            span {
                "Selected for compare: {selected().len()} / {COMPARE_LIMIT}"
            }
            button {
                disabled: selected().is_empty(),
                onclick: run_compare,
                " Compare"
            }
            button {
                disabled: selected().is_empty() && comparison().is_none(),
                onclick: clear_compare,
                " Clear"
            }
            if let Some(msg) = compare_error() {
                p { class: "err", "{msg}" }
            }
        }

        // Results list
        for svc in results().into_iter() {
            div { key: "{svc.id}", class: "card",
                div { class: "row",
                    input {
                        r#type: "checkbox",
                        checked: selected().contains(&svc.id),
                        onchange: {
                            let id = svc.id.clone();
                            move |_| toggle_compare(id.clone())
                        },
                    }
                    strong { "{svc.name}" }
                    span { " · ${svc.price:.2} · ★{svc.rating:.1} · {svc.zip_code}" }
                }
                p { class: "muted", "{svc.description}" }
                button {
                    onclick: {
                        let id = svc.id.clone();
                        move |_| fav(id.clone())
                    },
                    "Favorite"
                }
            }
        }

        // Side-by-side comparison panel (uses .compare-grid for responsive wrap)
        if let Some(items) = comparison() {
            h3 { "Comparison" }
            div { class: "compare-grid",
                for item in items.into_iter() {
                    div { key: "{item.service.id}", class: "card",
                        h4 { "{item.service.name}" }
                        p { "Price: ${item.service.price:.2}" }
                        p { "Rating: ★{item.service.rating:.1}" }
                        p { "ZIP: {item.service.zip_code}" }
                        p {
                            "Tags: "
                            if item.tags.is_empty() {
                                span { class: "muted", "—" }
                            } else {
                                {
                                    item.tags
                                        .iter()
                                        .map(|t| t.name.clone())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                }
                            }
                        }
                        p {
                            "Availability windows: {item.availability.len()}"
                        }
                        for win in item.availability.iter() {
                            div { key: "{win.id}", class: "muted",
                                "{win.start_time} → {win.end_time}"
                            }
                        }
                    }
                }
            }
        }
    }
}
