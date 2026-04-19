use dioxus::prelude::*;
use shared::{FaceRecordDetail, Role};

use crate::api::face;
use crate::auth::use_auth;

#[component]
pub fn Face() -> Element {
    let auth = use_auth();
    let (user_id, role) = {
        let s = auth.0.read();
        let uid = s.user.as_ref().map(|u| u.id.clone()).unwrap_or_default();
        let r = s.user.as_ref().map(|u| u.role).unwrap_or(Role::Requester);
        (uid, r)
    };

    let mut records = use_signal(Vec::<FaceRecordDetail>::new);
    let mut error = use_signal(|| None::<String>);

    let uid_for_load = user_id.clone();
    use_future(move || {
        let uid = uid_for_load.clone();
        async move {
            match face::list_for_user(&uid).await {
                Ok(list) => records.set(list),
                Err(e) => error.set(Some(e)),
            }
        }
    });

    // `deactivate` is called from each row's onclick; the RSX builder is
    // `FnMut`, so capture it with each click via a fresh clone of the id
    // and inline the spawn. This avoids moving a non-`Fn` closure out of
    // a `FnMut` context.

    rsx! {
        h2 { "Face Records" }
        if let Some(msg) = error() {
            p { class: "err", "{msg}" }
        }

        p { class: "muted",
            "To register a new face image, POST a multipart form (field 'file') "
            "to /api/faces with your bearer token."
        }

        if records().is_empty() {
            p { class: "muted", "No face records yet." }
        }
        for rec in records().into_iter() {
            div { key: "{rec.record.id}", class: "card",
                strong { "Version {rec.record.version}" }
                " · "
                if rec.record.is_active {
                    span { "active" }
                } else {
                    span { class: "muted", "inactive" }
                }
                " · created {rec.record.created_at}"

                p { class: "muted", "Images: {rec.images.len()} · Audit entries: {rec.audits.len()}" }
                for img in rec.images.iter() {
                    div { key: "{img.id}", class: "muted",
                        "{img.resolution} · brightness {img.brightness_score:.1} · blur {img.blur_score:.0}"
                    }
                }
                if role == Role::Administrator && rec.record.is_active {
                    button {
                        onclick: {
                            let id = rec.record.id.clone();
                            let uid = user_id.clone();
                            move |_| {
                                let id = id.clone();
                                let uid = uid.clone();
                                spawn(async move {
                                    let _ = face::deactivate(&id).await;
                                    if let Ok(list) = face::list_for_user(&uid).await {
                                        records.set(list);
                                    }
                                });
                            }
                        },
                        "Deactivate"
                    }
                }
            }
        }
    }
}
