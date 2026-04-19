// Requester-side work order & review UI (initial + follow-up, image
// upload, tag selection). Role-aware: only requesters see the create-order
// form; completion is surfaced to service managers / admins.
//
// Reviews:
//   * "Create initial review" posts to /api/reviews with selected tag ids.
//   * "Submit follow-up" posts to /api/work-orders/<id>/follow-up-review.
//   * Images can be uploaded for either review (same endpoint by review id).

use dioxus::prelude::*;
use shared::{Review, ReviewKind, ReviewTag, Role, WorkOrder, WorkOrderStatus};
use wasm_bindgen::JsCast;

use crate::api::workorders;
use crate::auth::use_auth;

const IMAGE_INPUT_ID: &str = "wo-review-image-input";

fn read_selected_file(input_id: &str) -> Option<web_sys::File> {
    let doc = web_sys::window()?.document()?;
    let el = doc.get_element_by_id(input_id)?;
    let input: web_sys::HtmlInputElement = el.dyn_into().ok()?;
    input.files()?.get(0)
}

#[component]
pub fn WorkOrders() -> Element {
    let role = use_auth()
        .0
        .read()
        .user
        .as_ref()
        .map(|u| u.role)
        .unwrap_or(Role::Requester);

    let mut service_id = use_signal(String::new);
    let mut lookup_id = use_signal(String::new);
    let mut current = use_signal(|| None::<WorkOrder>);
    let mut error = use_signal(|| None::<String>);
    let mut info = use_signal(|| None::<String>);

    // Review form state. Shared between initial and follow-up: the
    // submitter picks the flow at submit time via separate buttons.
    let mut rating = use_signal(|| 5u8);
    let mut text = use_signal(String::new);
    let mut selected_tags = use_signal(Vec::<String>::new);
    let mut tags = use_signal(Vec::<ReviewTag>::new);
    let mut last_review = use_signal(|| None::<Review>);

    // Image upload target: id of the most recently created review (either
    // initial or follow-up).
    let mut image_target = use_signal(String::new);
    let mut image_status = use_signal(|| None::<String>);

    use_future(move || async move {
        if let Ok(list) = workorders::list_review_tags().await {
            tags.set(list);
        }
    });

    let mut clear_messages = move || {
        error.set(None);
        info.set(None);
    };

    let create_order = move |_| {
        let sid = service_id();
        if sid.trim().is_empty() {
            error.set(Some("service_id required".into()));
            return;
        }
        spawn(async move {
            clear_messages();
            match workorders::create_order(sid).await {
                Ok(wo) => {
                    info.set(Some(format!("Created work order {}", wo.id)));
                    current.set(Some(wo));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let load_order = move |_| {
        let id = lookup_id();
        spawn(async move {
            clear_messages();
            match workorders::get_order(&id).await {
                Ok(wo) => {
                    current.set(Some(wo));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let complete = move |_| {
        let Some(wo) = current() else {
            return;
        };
        let id = wo.id.clone();
        spawn(async move {
            clear_messages();
            match workorders::complete_order(&id).await {
                Ok(wo) => {
                    info.set(Some("Work order completed".into()));
                    current.set(Some(wo));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let mut toggle_tag = move |tid: String| {
        let mut sel = selected_tags.write();
        if let Some(pos) = sel.iter().position(|x| x == &tid) {
            sel.remove(pos);
        } else {
            sel.push(tid);
        }
    };

    let submit_initial = move |_| {
        let Some(wo) = current() else {
            error.set(Some("Load a work order first".into()));
            return;
        };
        let r = rating();
        let t = text();
        let sel = selected_tags();
        spawn(async move {
            clear_messages();
            match workorders::create_initial_review(wo.id.clone(), r, t, sel).await {
                Ok(rev) => {
                    info.set(Some(format!("Initial review created ({})", rev.id)));
                    image_target.set(rev.id.clone());
                    last_review.set(Some(rev));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let submit_follow_up = move |_| {
        let Some(wo) = current() else {
            error.set(Some("Load a work order first".into()));
            return;
        };
        let r = rating();
        let t = text();
        let sel = selected_tags();
        let wid = wo.id.clone();
        spawn(async move {
            clear_messages();
            match workorders::create_follow_up_review(&wid, r, t, sel).await {
                Ok(rev) => {
                    info.set(Some(format!("Follow-up review submitted ({})", rev.id)));
                    image_target.set(rev.id.clone());
                    last_review.set(Some(rev));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let do_upload = move |_| {
        let target = image_target();
        if target.is_empty() {
            image_status.set(Some("Submit a review first to get a review id".into()));
            return;
        }
        let Some(file) = read_selected_file(IMAGE_INPUT_ID) else {
            image_status.set(Some("No file selected".into()));
            return;
        };
        spawn(async move {
            match workorders::upload_review_image(&target, &file).await {
                Ok(img) => image_status.set(Some(format!(
                    "Uploaded image {} ({} bytes, {})",
                    img.id, img.size, img.content_type
                ))),
                Err(e) => image_status.set(Some(e)),
            }
        });
    };

    rsx! {
        h2 { "Work orders & reviews" }
        if let Some(m) = info() { p { class: "muted", "{m}" } }
        if let Some(m) = error() { p { class: "err", "{m}" } }

        // Requester: create a work order.
        if matches!(role, Role::Requester | Role::Administrator) {
            div { class: "card",
                h3 { "New work order" }
                div { class: "row",
                    label { "Service id" }
                    input {
                        value: "{service_id}",
                        oninput: move |e| service_id.set(e.value()),
                    }
                    button { onclick: create_order, "Create" }
                }
            }
        }

        // Look up an order (by id) to act on it.
        div { class: "card",
            h3 { "Open existing order" }
            div { class: "row",
                label { "Order id" }
                input {
                    value: "{lookup_id}",
                    oninput: move |e| lookup_id.set(e.value()),
                }
                button { onclick: load_order, "Open" }
            }
        }

        // Current order + actions.
        if let Some(wo) = current() {
            div { class: "card",
                h3 { "Order {wo.id}" }
                p { class: "muted",
                    "Service: {wo.service_id} · Status: "
                    if wo.status == WorkOrderStatus::Completed { "completed" }
                    else if wo.status == WorkOrderStatus::Pending { "pending" }
                    else if wo.status == WorkOrderStatus::InProgress { "in progress" }
                    else { "cancelled" }
                }
                if matches!(role, Role::Administrator | Role::ServiceManager)
                    && wo.status != WorkOrderStatus::Completed
                {
                    button { onclick: complete, "Mark completed" }
                }
            }

            // Review submission — relevant only to the requester on a
            // completed order. UI still renders the controls as a safety
            // net; backend rejects non-requester / non-completed attempts.
            div { class: "card",
                h3 { "Review" }
                div { class: "row",
                    label { "Rating (1–5)" }
                    input {
                        r#type: "number",
                        min: "1",
                        max: "5",
                        value: "{rating}",
                        oninput: move |e| {
                            if let Ok(n) = e.value().parse::<u8>() {
                                rating.set(n.clamp(1, 5));
                            }
                        },
                    }
                }
                div { class: "row",
                    textarea {
                        placeholder: "Review text",
                        rows: "4",
                        value: "{text}",
                        oninput: move |e| text.set(e.value()),
                    }
                }
                if tags().is_empty() {
                    p { class: "muted", "No review tags configured yet." }
                } else {
                    p { "Tags:" }
                    div { class: "row",
                        for t in tags().into_iter() {
                            label { key: "{t.id}",
                                input {
                                    r#type: "checkbox",
                                    checked: selected_tags().contains(&t.id),
                                    onchange: {
                                        let tid = t.id.clone();
                                        move |_| toggle_tag(tid.clone())
                                    },
                                }
                                " {t.name}"
                            }
                        }
                    }
                }
                div { class: "row",
                    button { onclick: submit_initial, "Submit initial review" }
                    button { onclick: submit_follow_up, "Submit follow-up" }
                }
            }

            // Image upload (applies to whichever review was just created).
            div { class: "card",
                h3 { "Attach image" }
                if let Some(rev) = last_review() {
                    p { class: "muted",
                        "Target review "
                        if rev.kind == ReviewKind::FollowUp { "(follow-up) " } else { "(initial) " }
                        "{rev.id}"
                    }
                } else {
                    p { class: "muted", "Submit a review first to upload images." }
                }
                input {
                    id: "{IMAGE_INPUT_ID}",
                    r#type: "file",
                    accept: "image/png,image/jpeg",
                }
                button { onclick: do_upload, "Upload" }
                if let Some(msg) = image_status() { p { class: "muted", "{msg}" } }
            }
        }
    }
}
