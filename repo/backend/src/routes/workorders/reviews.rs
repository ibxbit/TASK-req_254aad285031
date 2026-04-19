use chrono::{Duration, Utc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{
    AssignReviewTagRequest, CollapseReviewRequest, CreateFollowUpReviewRequest,
    CreateReviewRequest, CreateReviewTagRequest, PinReviewRequest, Review, ReviewKind, ReviewTag,
    Role,
};
use sqlx::{MySql, MySqlPool, Transaction};
use uuid::Uuid;

use crate::audit;
use crate::auth::guard::AuthUser;
use crate::workorders::{MAX_REVIEWS_PER_DAY, REVIEW_WINDOW_DAYS};

const ENTITY: &str = "review";

// Shared sanity check for a posted rating.
fn check_rating(r: u8) -> Result<(), Status> {
    if !(1..=5).contains(&r) {
        Err(Status::BadRequest)
    } else {
        Ok(())
    }
}

// Resolve and attach requester-selected tag ids inside the same transaction
// as the review insert. Unknown tag ids -> 400 (we don't silently drop,
// otherwise the audit record would claim tags that weren't persisted).
async fn attach_selected_tags(
    tx: &mut Transaction<'_, MySql>,
    review_id: Uuid,
    tag_ids: &[String],
) -> Result<Vec<Uuid>, Status> {
    let mut parsed: Vec<Uuid> = Vec::with_capacity(tag_ids.len());
    for t in tag_ids {
        parsed.push(Uuid::parse_str(t).map_err(|_| Status::BadRequest)?);
    }
    // De-duplicate so the audit record is canonical.
    parsed.sort();
    parsed.dedup();

    for tid in &parsed {
        let exists: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM review_tags WHERE id = ? LIMIT 1")
                .bind(&tid.as_bytes()[..])
                .fetch_optional(&mut **tx)
                .await
                .map_err(|_| Status::InternalServerError)?;
        if exists.is_none() {
            return Err(Status::BadRequest);
        }
        sqlx::query("INSERT IGNORE INTO review_tag_map (review_id, tag_id) VALUES (?, ?)")
            .bind(&review_id.as_bytes()[..])
            .bind(&tid.as_bytes()[..])
            .execute(&mut **tx)
            .await
            .map_err(|_| Status::InternalServerError)?;
    }
    Ok(parsed)
}

// Enforce the per-user daily cap (applies to initial and follow-up equally,
// since the cap is about user behaviour, not review kind).
async fn assert_daily_cap(pool: &MySqlPool, user_id: Uuid) -> Result<(), Status> {
    let count_today: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM reviews \
         WHERE user_id = ? AND DATE(created_at) = CURDATE()",
    )
    .bind(&user_id.as_bytes()[..])
    .fetch_one(pool)
    .await
    .map_err(|_| Status::InternalServerError)?;
    if count_today.0 >= MAX_REVIEWS_PER_DAY {
        return Err(Status::TooManyRequests);
    }
    Ok(())
}

// Review creation validation (spec-ordered):
//   1. Work order exists, is completed, belongs to this requester
//   2. No INITIAL review already exists for this work order (409)
//   3. NOW <= completed_at + 14 days (410 Gone if window closed)
//   4. User has < 3 reviews today (429)
// Image count (max 5) is enforced at POST /reviews/{id}/images.
// Requester-selected tag ids are validated and attached atomically.
#[post("/reviews", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateReviewRequest>,
) -> Result<Json<Review>, Status> {
    check_rating(req.rating)?;
    let wid = Uuid::parse_str(&req.work_order_id).map_err(|_| Status::BadRequest)?;

    let wo: Option<(Vec<u8>, String, Option<chrono::NaiveDateTime>)> =
        sqlx::query_as("SELECT requester_id, status, completed_at FROM work_orders WHERE id = ?")
            .bind(&wid.as_bytes()[..])
            .fetch_optional(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;
    let Some((req_b, status, completed_at)) = wo else {
        return Err(Status::NotFound);
    };
    let requester = Uuid::from_slice(&req_b).map_err(|_| Status::InternalServerError)?;
    if requester != user.id {
        return Err(Status::Forbidden);
    }
    if status != "completed" {
        return Err(Status::BadRequest);
    }
    let Some(completed) = completed_at else {
        return Err(Status::BadRequest);
    };

    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM reviews WHERE work_order_id = ? AND kind = 'initial' LIMIT 1",
    )
    .bind(&wid.as_bytes()[..])
    .fetch_optional(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    if existing.is_some() {
        return Err(Status::Conflict);
    }

    let now = Utc::now().naive_utc();
    if now > completed + Duration::days(REVIEW_WINDOW_DAYS) {
        return Err(Status::Gone);
    }

    assert_daily_cap(pool.inner(), user.id).await?;

    let rid = Uuid::new_v4();
    let created_at = now;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let insert_res = sqlx::query(
        "INSERT INTO reviews \
         (id, work_order_id, user_id, rating, text, kind, parent_review_id, created_at) \
         VALUES (?, ?, ?, ?, ?, 'initial', NULL, ?)",
    )
    .bind(&rid.as_bytes()[..])
    .bind(&wid.as_bytes()[..])
    .bind(&user.id.as_bytes()[..])
    .bind(req.rating)
    .bind(&req.text)
    .bind(created_at)
    .execute(&mut *tx)
    .await;

    if let Err(sqlx::Error::Database(e)) = &insert_res {
        if e.is_unique_violation() {
            return Err(Status::Conflict);
        }
    }
    insert_res.map_err(|_| Status::InternalServerError)?;

    let tag_ids = attach_selected_tags(&mut tx, rid, &req.tag_ids).await?;

    let payload = serde_json::json!({
        "action": "create",
        "id": rid.to_string(),
        "work_order_id": wid.to_string(),
        "user_id": user.id.to_string(),
        "rating": req.rating,
        "text": req.text,
        "kind": "initial",
        "tag_ids": tag_ids.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
        "created_at": created_at.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, rid, "create", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(Review {
        id: rid.to_string(),
        work_order_id: wid.to_string(),
        user_id: user.id.to_string(),
        rating: req.rating,
        text: req.text.clone(),
        is_pinned: false,
        is_collapsed: false,
        kind: ReviewKind::Initial,
        parent_review_id: None,
        created_at,
    }))
}

// Follow-up review — exactly one per completed work order, only after the
// initial review already exists, by the same requester, within the same
// 14-day window and 3/day cap. Image upload reuses the same endpoint
// (/reviews/<id>/images) with the follow-up review's id.
#[post("/work-orders/<id>/follow-up-review", format = "json", data = "<req>")]
pub async fn create_follow_up(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<CreateFollowUpReviewRequest>,
) -> Result<Json<Review>, Status> {
    check_rating(req.rating)?;
    let wid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let wo: Option<(Vec<u8>, String, Option<chrono::NaiveDateTime>)> =
        sqlx::query_as("SELECT requester_id, status, completed_at FROM work_orders WHERE id = ?")
            .bind(&wid.as_bytes()[..])
            .fetch_optional(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;
    let Some((req_b, status, completed_at)) = wo else {
        return Err(Status::NotFound);
    };
    let requester = Uuid::from_slice(&req_b).map_err(|_| Status::InternalServerError)?;
    if requester != user.id {
        return Err(Status::Forbidden);
    }
    if status != "completed" {
        return Err(Status::BadRequest);
    }
    let Some(completed) = completed_at else {
        return Err(Status::BadRequest);
    };

    // Parent = the initial review. Required before a follow-up can exist.
    let parent: Option<(Vec<u8>,)> = sqlx::query_as(
        "SELECT id FROM reviews WHERE work_order_id = ? AND kind = 'initial' LIMIT 1",
    )
    .bind(&wid.as_bytes()[..])
    .fetch_optional(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    let Some((parent_b,)) = parent else {
        // No initial review yet -> follow-up is a precondition failure.
        return Err(Status::Conflict);
    };
    let parent_id = Uuid::from_slice(&parent_b).map_err(|_| Status::InternalServerError)?;

    // One follow-up per order.
    let existing_followup: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM reviews WHERE work_order_id = ? AND kind = 'follow_up' LIMIT 1",
    )
    .bind(&wid.as_bytes()[..])
    .fetch_optional(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    if existing_followup.is_some() {
        return Err(Status::Conflict);
    }

    let now = Utc::now().naive_utc();
    if now > completed + Duration::days(REVIEW_WINDOW_DAYS) {
        return Err(Status::Gone);
    }

    assert_daily_cap(pool.inner(), user.id).await?;

    let rid = Uuid::new_v4();
    let created_at = now;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let insert_res = sqlx::query(
        "INSERT INTO reviews \
         (id, work_order_id, user_id, rating, text, kind, parent_review_id, created_at) \
         VALUES (?, ?, ?, ?, ?, 'follow_up', ?, ?)",
    )
    .bind(&rid.as_bytes()[..])
    .bind(&wid.as_bytes()[..])
    .bind(&user.id.as_bytes()[..])
    .bind(req.rating)
    .bind(&req.text)
    .bind(&parent_id.as_bytes()[..])
    .bind(created_at)
    .execute(&mut *tx)
    .await;

    if let Err(sqlx::Error::Database(e)) = &insert_res {
        if e.is_unique_violation() {
            return Err(Status::Conflict);
        }
    }
    insert_res.map_err(|_| Status::InternalServerError)?;

    let tag_ids = attach_selected_tags(&mut tx, rid, &req.tag_ids).await?;

    let payload = serde_json::json!({
        "action": "follow_up",
        "id": rid.to_string(),
        "work_order_id": wid.to_string(),
        "user_id": user.id.to_string(),
        "rating": req.rating,
        "text": req.text,
        "kind": "follow_up",
        "parent_review_id": parent_id.to_string(),
        "tag_ids": tag_ids.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
        "created_at": created_at.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, rid, "follow_up", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(Review {
        id: rid.to_string(),
        work_order_id: wid.to_string(),
        user_id: user.id.to_string(),
        rating: req.rating,
        text: req.text.clone(),
        is_pinned: false,
        is_collapsed: false,
        kind: ReviewKind::FollowUp,
        parent_review_id: Some(parent_id.to_string()),
        created_at,
    }))
}

#[get("/services/<id>/reviews")]
pub async fn list_for_service(
    pool: &State<MySqlPool>,
    _user: AuthUser,
    id: &str,
) -> Result<Json<Vec<Review>>, Status> {
    let sid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let rows: Vec<(
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        u8,
        String,
        i8,
        i8,
        String,
        Option<Vec<u8>>,
        chrono::NaiveDateTime,
    )> = sqlx::query_as(
        "SELECT r.id, r.work_order_id, r.user_id, r.rating, r.text, \
                r.is_pinned, r.is_collapsed, r.kind, r.parent_review_id, r.created_at \
         FROM reviews r \
         JOIN work_orders wo ON wo.id = r.work_order_id \
         WHERE wo.service_id = ? \
         ORDER BY r.is_pinned DESC, r.created_at DESC, r.id",
    )
    .bind(&sid.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    let reviews = rows
        .into_iter()
        .filter_map(
            |(rid, wid, uid, rating, text, pin, col, kind, parent, ts)| {
                Some(Review {
                    id: Uuid::from_slice(&rid).ok()?.to_string(),
                    work_order_id: Uuid::from_slice(&wid).ok()?.to_string(),
                    user_id: Uuid::from_slice(&uid).ok()?.to_string(),
                    rating,
                    text,
                    is_pinned: pin != 0,
                    is_collapsed: col != 0,
                    kind: ReviewKind::from_str(&kind).unwrap_or(ReviewKind::Initial),
                    parent_review_id: parent
                        .and_then(|b| Uuid::from_slice(&b).ok().map(|u| u.to_string())),
                    created_at: ts,
                })
            },
        )
        .collect();
    Ok(Json(reviews))
}

#[patch("/reviews/<id>/pin", format = "json", data = "<req>")]
pub async fn pin(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<PinReviewRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let rid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let res = sqlx::query("UPDATE reviews SET is_pinned = ? WHERE id = ?")
        .bind(req.is_pinned as i8)
        .bind(&rid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?;
    if res.rows_affected() == 0 {
        return Err(Status::NotFound);
    }

    let payload = serde_json::json!({
        "action": "pin",
        "id": rid.to_string(),
        "is_pinned": req.is_pinned,
    });
    audit::record_event_tx(&mut tx, ENTITY, rid, "pin", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

#[patch("/reviews/<id>/collapse", format = "json", data = "<req>")]
pub async fn collapse(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<CollapseReviewRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let rid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let res = sqlx::query("UPDATE reviews SET is_collapsed = ? WHERE id = ?")
        .bind(req.is_collapsed as i8)
        .bind(&rid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?;
    if res.rows_affected() == 0 {
        return Err(Status::NotFound);
    }

    let payload = serde_json::json!({
        "action": "collapse",
        "id": rid.to_string(),
        "is_collapsed": req.is_collapsed,
    });
    audit::record_event_tx(&mut tx, ENTITY, rid, "collapse", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

// ---------- Review tags ----------
// Creation of tag vocabulary stays admin-only (governance).
// Attaching to a review has two surfaces:
//   * requester-selectable at create time (in CreateReviewRequest.tag_ids)
//   * admin tagging after the fact (POST /reviews/<id>/tags below).
// Both paths write to review_tag_map and are covered by the audit chain.

#[post("/review-tags", format = "json", data = "<req>")]
pub async fn create_tag(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateReviewTagRequest>,
) -> Result<Json<ReviewTag>, Status> {
    user.require_role(Role::Administrator)?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO review_tags (id, name) VALUES (?, ?)")
        .bind(&id.as_bytes()[..])
        .bind(&req.name)
        .execute(pool.inner())
        .await
        .map_err(|_| Status::BadRequest)?;
    Ok(Json(ReviewTag {
        id: id.to_string(),
        name: req.name.clone(),
    }))
}

#[get("/review-tags")]
pub async fn list_tags(
    pool: &State<MySqlPool>,
    _user: AuthUser,
) -> Result<Json<Vec<ReviewTag>>, Status> {
    let rows: Vec<(Vec<u8>, String)> =
        sqlx::query_as("SELECT id, name FROM review_tags ORDER BY name, id")
            .fetch_all(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;
    let tags = rows
        .into_iter()
        .filter_map(|(id, name)| {
            Some(ReviewTag {
                id: Uuid::from_slice(&id).ok()?.to_string(),
                name,
            })
        })
        .collect();
    Ok(Json(tags))
}

// Admin tag assignment (retains existing admin-tagging workflow in addition
// to the new requester-selectable path at review creation).
#[post("/reviews/<id>/tags", format = "json", data = "<req>")]
pub async fn assign_tag(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<AssignReviewTagRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let rid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let tid = Uuid::parse_str(&req.tag_id).map_err(|_| Status::BadRequest)?;

    // Guard: tag must exist and review must exist. Prevents silent inserts
    // of orphan mappings (FKs cover it, but explicit 404 is clearer).
    let review_exists: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM reviews WHERE id = ? LIMIT 1")
            .bind(&rid.as_bytes()[..])
            .fetch_optional(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;
    if review_exists.is_none() {
        return Err(Status::NotFound);
    }

    let tag_exists: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM review_tags WHERE id = ? LIMIT 1")
            .bind(&tid.as_bytes()[..])
            .fetch_optional(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;
    if tag_exists.is_none() {
        return Err(Status::NotFound);
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    sqlx::query("INSERT IGNORE INTO review_tag_map (review_id, tag_id) VALUES (?, ?)")
        .bind(&rid.as_bytes()[..])
        .bind(&tid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::BadRequest)?;

    let payload = serde_json::json!({
        "action": "tag",
        "id": rid.to_string(),
        "tag_id": tid.to_string(),
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, rid, "tag", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Status::Created)
}
