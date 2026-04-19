// Deterministic reputation scoring with 180-day exponential decay.
//
// Per spec:
//   weight(r) = exp(-days_since_review / 180)
//   final_score = SUM(rating * weight) / SUM(weight)
//
// Traceability:
//   * Reviews are fetched with a fixed ordering (created_at, id) so f64 sums
//     are reduced in the same order every call.
//   * "NOW" is captured once per request and reused for every review so a
//     single call is self-consistent.
//   * Appending `?breakdown=true` returns per-review (days_since, weight)
//     rows — the same numbers used internally, so a caller can recompute the
//     final_score offline and verify no hidden logic was applied.
//   * Only non-collapsed reviews contribute; collapsed reviews are omitted
//     from both the numerator and the count.

use chrono::{NaiveDateTime, Utc};
use rocket::form::FromForm;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{Reputation, ReputationBreakdownEntry};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::auth::guard::AuthUser;
use crate::workorders::DECAY_DAYS;

const SECONDS_PER_DAY: f64 = 86_400.0;

fn weight_for(days_since: f64) -> f64 {
    // Clamp against negative ages (clock skew) so weight never exceeds 1.
    (-days_since.max(0.0) / DECAY_DAYS).exp()
}

#[derive(FromForm)]
pub struct ReputationQuery {
    pub breakdown: Option<bool>,
}

#[get("/services/<id>/reputation?<query..>")]
pub async fn get(
    pool: &State<MySqlPool>,
    _user: AuthUser,
    id: &str,
    query: ReputationQuery,
) -> Result<Json<Reputation>, Status> {
    let sid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    // Step 1: fetch all non-collapsed reviews for the service, stable order.
    let rows: Vec<(Vec<u8>, u8, NaiveDateTime)> = sqlx::query_as(
        "SELECT r.id, r.rating, r.created_at \
         FROM reviews r \
         JOIN work_orders wo ON wo.id = r.work_order_id \
         WHERE wo.service_id = ? AND r.is_collapsed = 0 \
         ORDER BY r.created_at, r.id",
    )
    .bind(&sid.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    // Single "NOW" reused for every review so the result is self-consistent.
    let now = Utc::now().naive_utc();
    let include_breakdown = query.breakdown.unwrap_or(false);

    let mut numerator: f64 = 0.0;
    let mut denominator: f64 = 0.0;
    let mut breakdown: Vec<ReputationBreakdownEntry> = if include_breakdown {
        Vec::with_capacity(rows.len())
    } else {
        Vec::new()
    };

    for (id_b, rating, created_at) in &rows {
        // Step 2: compute fractional days since the review.
        let delta = *now_ref(&now) - *created_at;
        let days_since = delta.num_seconds() as f64 / SECONDS_PER_DAY;

        // Step 3: apply exponential decay.
        let w = weight_for(days_since);

        // Step 4: accumulate weighted average.
        numerator += (*rating as f64) * w;
        denominator += w;

        if include_breakdown {
            breakdown.push(ReputationBreakdownEntry {
                review_id: Uuid::from_slice(id_b)
                    .map(|u| u.to_string())
                    .unwrap_or_default(),
                rating: *rating,
                days_since: days_since.max(0.0),
                weight: w,
                created_at: *created_at,
            });
        }
    }

    let final_score = if denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    };

    Ok(Json(Reputation {
        service_id: sid.to_string(),
        final_score,
        total_reviews: rows.len() as i64,
        breakdown: if include_breakdown {
            Some(breakdown)
        } else {
            None
        },
    }))
}

// Small helper so `now - created_at` reads linearly; avoids cloning NaiveDateTime.
fn now_ref(n: &NaiveDateTime) -> &NaiveDateTime {
    n
}
