use image::GenericImageView;
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use shared::{
    FaceAudit, FaceCheckResult, FaceImage, FaceLivenessChallenge, FaceRecord, FaceRecordDetail,
    FaceValidationResult, Role,
};
use sqlx::MySqlPool;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::audit;
use crate::auth::guard::AuthUser;
use crate::config::StorageConfig;
use crate::face::{
    analyze_frontal_face, compute_metrics, content_hash_hex, hamming_distance,
    parse_perceptual_hex, perceptual_hash_hex, run_checks, DEDUP_HAMMING_THRESHOLD,
};
use crate::logging;

const ENTITY: &str = "face_record";

#[derive(FromForm)]
pub struct FaceUploadForm<'r> {
    pub file: TempFile<'r>,
}

async fn read_temp_file(file: &mut TempFile<'_>) -> std::io::Result<Vec<u8>> {
    let path = match file.path() {
        Some(p) => p.to_path_buf(),
        None => return Err(std::io::Error::other("no temp path")),
    };
    let mut f = tokio::fs::File::open(&path).await?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).await?;
    Ok(buf)
}

// ---------- Capture / import ----------

#[post("/faces", data = "<upload>")]
pub async fn create(
    pool: &State<MySqlPool>,
    storage_cfg: &State<StorageConfig>,
    user: AuthUser,
    upload: Form<FaceUploadForm<'_>>,
) -> Result<Json<FaceRecord>, Status> {
    let mut form = upload.into_inner();

    let ct = form
        .file
        .content_type()
        .ok_or(Status::UnsupportedMediaType)?;
    let ext = if ct.is_jpeg() {
        "jpg"
    } else if ct.is_png() {
        "png"
    } else {
        return Err(Status::UnsupportedMediaType);
    };

    let bytes = read_temp_file(&mut form.file)
        .await
        .map_err(|_| Status::InternalServerError)?;
    if bytes.is_empty() {
        return Err(Status::BadRequest);
    }

    let img = image::load_from_memory(&bytes).map_err(|_| Status::BadRequest)?;
    let (w, h) = img.dimensions();

    if w < crate::face::MIN_WIDTH || h < crate::face::MIN_HEIGHT {
        logging::validation_failed("POST /faces", "resolution below minimum");
        return Err(Status::UnprocessableEntity);
    }

    let (brightness, blur_variance) = compute_metrics(&img);
    let frontal = analyze_frontal_face(&img);
    let content_hex = content_hash_hex(&bytes);
    let phash_hex = perceptual_hash_hex(&img);

    // Enforce all mandatory capture-time checks; no fallback "always-pass"
    // branch exists. This is identical to the validate() path — if the
    // image would fail validation later, reject at capture time instead
    // of storing junk.
    let checks = run_checks(w, h, brightness, blur_variance, Some(&frontal));
    if !checks.iter().all(|c| c.passed) {
        logging::validation_failed("POST /faces", "face capture failed mandatory checks");
        return Err(Status::UnprocessableEntity);
    }

    let existing_hashes: Vec<(String,)> = sqlx::query_as(
        "SELECT fi.perceptual_hash FROM face_images fi \
         JOIN face_records fr ON fr.id = fi.face_record_id \
         WHERE fr.user_id = ?",
    )
    .bind(&user.id.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    let new_hash = parse_perceptual_hex(&phash_hex).ok_or(Status::InternalServerError)?;
    for (h,) in &existing_hashes {
        if let Some(existing) = parse_perceptual_hex(h) {
            if hamming_distance(new_hash, existing) <= DEDUP_HAMMING_THRESHOLD {
                return Err(Status::Conflict);
            }
        }
    }

    let image_id = Uuid::new_v4();
    let filename = format!("{}.{}", image_id, ext);
    let dir = std::path::PathBuf::from(&storage_cfg.face_images_dir);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let path = dir.join(&filename);
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let rel_path = path.to_string_lossy().to_string();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let next_version: (Option<i32>,) =
        sqlx::query_as("SELECT MAX(version) FROM face_records WHERE user_id = ?")
            .bind(&user.id.as_bytes()[..])
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| Status::InternalServerError)?;
    let version = next_version.0.unwrap_or(0) + 1;

    let prior_active: Vec<(Vec<u8>,)> = sqlx::query_as(
        "SELECT id FROM face_records WHERE user_id = ? AND is_active = 1 FOR UPDATE",
    )
    .bind(&user.id.as_bytes()[..])
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| Status::InternalServerError)?;

    for (old_id_b,) in &prior_active {
        let old_id = Uuid::from_slice(old_id_b).map_err(|_| Status::InternalServerError)?;

        sqlx::query("UPDATE face_records SET is_active = 0 WHERE id = ?")
            .bind(&old_id.as_bytes()[..])
            .execute(&mut *tx)
            .await
            .map_err(|_| Status::InternalServerError)?;

        let aid_old = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO face_audits (id, face_record_id, action, performed_by) \
             VALUES (?, ?, 'deactivated', ?)",
        )
        .bind(&aid_old.as_bytes()[..])
        .bind(&old_id.as_bytes()[..])
        .bind(&user.id.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?;

        let payload_old = serde_json::json!({
            "action": "deactivated_by_new_version",
            "id": old_id.to_string(),
            "superseded_by_version": version,
        });
        audit::record_event_tx(&mut tx, ENTITY, old_id, "deactivate", &payload_old)
            .await
            .map_err(|_| Status::InternalServerError)?;
    }

    let record_id = Uuid::new_v4();
    sqlx::query("INSERT INTO face_records (id, user_id, version, is_active) VALUES (?, ?, ?, 1)")
        .bind(&record_id.as_bytes()[..])
        .bind(&user.id.as_bytes()[..])
        .bind(version)
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?;

    sqlx::query(
        "INSERT INTO face_images \
         (id, face_record_id, file_path, hash, perceptual_hash, resolution, \
          brightness_score, blur_score) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&image_id.as_bytes()[..])
    .bind(&record_id.as_bytes()[..])
    .bind(&rel_path)
    .bind(&content_hex)
    .bind(&phash_hex)
    .bind(format!("{}x{}", w, h))
    .bind(brightness)
    .bind(blur_variance)
    .execute(&mut *tx)
    .await
    .map_err(|_| Status::InternalServerError)?;

    let aid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO face_audits (id, face_record_id, action, performed_by) \
         VALUES (?, ?, 'created', ?)",
    )
    .bind(&aid.as_bytes()[..])
    .bind(&record_id.as_bytes()[..])
    .bind(&user.id.as_bytes()[..])
    .execute(&mut *tx)
    .await
    .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "create",
        "id": record_id.to_string(),
        "user_id": user.id.to_string(),
        "version": version,
        "image_id": image_id.to_string(),
        "content_hash": content_hex,
        "perceptual_hash": phash_hex,
        "resolution": format!("{}x{}", w, h),
        "brightness": brightness,
        "blur_variance": blur_variance,
        "frontal_face": {
            "skin_density": frontal.skin_density,
            "center_offset": frontal.center_offset,
            "lr_symmetry": frontal.lr_symmetry,
            "cluster_count": frontal.cluster_count,
        },
    });
    audit::record_event_tx(&mut tx, ENTITY, record_id, "create", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(FaceRecord {
        id: record_id.to_string(),
        user_id: user.id.to_string(),
        version,
        is_active: true,
        created_at: chrono::Utc::now().naive_utc(),
    }))
}

// ---------- Validate ----------

#[post("/faces/<id>/validate")]
pub async fn validate(
    pool: &State<MySqlPool>,
    storage_cfg: &State<StorageConfig>,
    user: AuthUser,
    id: &str,
) -> Result<Json<FaceValidationResult>, Status> {
    let rid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT user_id FROM face_records WHERE id = ?")
        .bind(&rid.as_bytes()[..])
        .fetch_optional(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    let Some((owner_b,)) = row else {
        return Err(Status::NotFound);
    };
    let owner = Uuid::from_slice(&owner_b).map_err(|_| Status::InternalServerError)?;
    if owner != user.id && user.role != Role::Administrator {
        logging::permission_denied(
            &user.id.to_string(),
            user.role.as_str(),
            "POST /faces/<id>/validate",
        );
        return Err(Status::Forbidden);
    }

    let img_row: Option<(String, f64, f64, String, String)> = sqlx::query_as(
        "SELECT resolution, brightness_score, blur_score, file_path, hash \
         FROM face_images WHERE face_record_id = ? \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&rid.as_bytes()[..])
    .fetch_optional(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    let Some((resolution, brightness, blur, file_path, stored_hash)) = img_row else {
        return Err(Status::NotFound);
    };

    let (w, h) = parse_resolution(&resolution).ok_or(Status::InternalServerError)?;

    // Re-load the image bytes to run the frontal-face analysis at
    // validation time. Storing the per-axis metrics would be cheaper but
    // re-analysis is the audit-honest answer: the validator sees the same
    // bytes a reviewer would see. Path stays under the configured dir.
    let _ = storage_cfg; // kept for symmetry with other handlers
    let bytes = match tokio::fs::read(&file_path).await {
        Ok(b) => b,
        Err(_) => return Err(Status::NotFound),
    };
    // Integrity check: the file must match the stored SHA-256.
    let current_hash = content_hash_hex(&bytes);
    if current_hash != stored_hash {
        logging::validation_failed("POST /faces/<id>/validate", "image content hash mismatch");
        return Err(Status::UnprocessableEntity);
    }
    let frontal = match image::load_from_memory(&bytes) {
        Ok(img) => Some(analyze_frontal_face(&img)),
        Err(_) => None,
    };

    let checks = run_checks(w, h, brightness, blur, frontal.as_ref());
    let passed = checks.iter().all(|c| c.passed);
    let action = if passed { "validated" } else { "rejected" };

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let aid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO face_audits (id, face_record_id, action, performed_by) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(&aid.as_bytes()[..])
    .bind(&rid.as_bytes()[..])
    .bind(action)
    .bind(&user.id.as_bytes()[..])
    .execute(&mut *tx)
    .await
    .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": action,
        "id": rid.to_string(),
        "passed": passed,
        "checks": checks.iter().map(|c| serde_json::json!({
            "name": c.name,
            "passed": c.passed,
            "message": c.message,
        })).collect::<Vec<_>>(),
    });
    audit::record_event_tx(&mut tx, ENTITY, rid, action, &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(FaceValidationResult {
        passed,
        checks: checks
            .into_iter()
            .map(|c| FaceCheckResult {
                name: c.name.to_string(),
                passed: c.passed,
                message: c.message,
            })
            .collect(),
    }))
}

fn parse_resolution(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

// ---------- Liveness challenge ----------

#[derive(Debug, Deserialize)]
pub struct RecordLivenessRequest {
    pub challenge: String,
    pub passed: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RecordLivenessResponse {
    pub id: String,
    pub face_record_id: String,
    pub challenge: String,
    pub passed: bool,
    pub created_at: chrono::NaiveDateTime,
}

const ALLOWED_CHALLENGES: &[&str] = &["turn_left", "turn_right", "blink", "nod", "smile", "tilt"];

#[post("/faces/<id>/liveness", format = "json", data = "<req>")]
pub async fn record_liveness(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<RecordLivenessRequest>,
) -> Result<Json<RecordLivenessResponse>, Status> {
    let rid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT user_id FROM face_records WHERE id = ?")
        .bind(&rid.as_bytes()[..])
        .fetch_optional(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    let Some((owner_b,)) = row else {
        return Err(Status::NotFound);
    };
    let owner = Uuid::from_slice(&owner_b).map_err(|_| Status::InternalServerError)?;
    if owner != user.id && user.role != Role::Administrator {
        return Err(Status::Forbidden);
    }

    if !ALLOWED_CHALLENGES.contains(&req.challenge.as_str()) {
        return Err(Status::BadRequest);
    }

    let chal_id = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    sqlx::query(
        "INSERT INTO face_liveness_challenges \
         (id, face_record_id, challenge, passed, notes, performed_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&chal_id.as_bytes()[..])
    .bind(&rid.as_bytes()[..])
    .bind(&req.challenge)
    .bind(req.passed as i8)
    .bind(&req.notes)
    .bind(&user.id.as_bytes()[..])
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "liveness",
        "id": chal_id.to_string(),
        "face_record_id": rid.to_string(),
        "challenge": req.challenge,
        "passed": req.passed,
        "performed_by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, rid, "liveness", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(RecordLivenessResponse {
        id: chal_id.to_string(),
        face_record_id: rid.to_string(),
        challenge: req.challenge.clone(),
        passed: req.passed,
        created_at: now,
    }))
}

// ---------- Deactivate ----------

#[post("/faces/<id>/deactivate")]
pub async fn deactivate(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let rid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let res = sqlx::query("UPDATE face_records SET is_active = 0 WHERE id = ? AND is_active = 1")
        .bind(&rid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?;

    if res.rows_affected() == 0 {
        return Err(Status::NotFound);
    }

    let aid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO face_audits (id, face_record_id, action, performed_by) \
         VALUES (?, ?, 'deactivated', ?)",
    )
    .bind(&aid.as_bytes()[..])
    .bind(&rid.as_bytes()[..])
    .bind(&user.id.as_bytes()[..])
    .execute(&mut *tx)
    .await
    .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "deactivate",
        "id": rid.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, rid, "deactivate", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

// ---------- List for user ----------

#[get("/faces/<user_id>")]
pub async fn list_for_user(
    pool: &State<MySqlPool>,
    user: AuthUser,
    user_id: &str,
) -> Result<Json<Vec<FaceRecordDetail>>, Status> {
    let target = Uuid::parse_str(user_id).map_err(|_| Status::BadRequest)?;
    if target != user.id && user.role != Role::Administrator {
        return Err(Status::Forbidden);
    }

    let records: Vec<(Vec<u8>, Vec<u8>, i32, i8, chrono::NaiveDateTime)> = sqlx::query_as(
        "SELECT id, user_id, version, is_active, created_at \
         FROM face_records WHERE user_id = ? ORDER BY version DESC",
    )
    .bind(&target.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    let mut out: Vec<FaceRecordDetail> = Vec::with_capacity(records.len());

    for (rid_b, uid_b, version, active, created_at) in records {
        let rid = Uuid::from_slice(&rid_b).map_err(|_| Status::InternalServerError)?;

        let images: Vec<(Vec<u8>, String, String, String, String, f64, f64)> = sqlx::query_as(
            "SELECT id, file_path, hash, perceptual_hash, resolution, \
                    brightness_score, blur_score \
             FROM face_images WHERE face_record_id = ? ORDER BY created_at",
        )
        .bind(&rid.as_bytes()[..])
        .fetch_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

        let audits: Vec<(Vec<u8>, String, Vec<u8>, chrono::NaiveDateTime)> = sqlx::query_as(
            "SELECT id, action, performed_by, created_at \
             FROM face_audits WHERE face_record_id = ? ORDER BY created_at",
        )
        .bind(&rid.as_bytes()[..])
        .fetch_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

        let liveness: Vec<(
            Vec<u8>,
            String,
            i8,
            Option<String>,
            Vec<u8>,
            chrono::NaiveDateTime,
        )> = sqlx::query_as(
            "SELECT id, challenge, passed, notes, performed_by, created_at \
             FROM face_liveness_challenges WHERE face_record_id = ? ORDER BY created_at",
        )
        .bind(&rid.as_bytes()[..])
        .fetch_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

        out.push(FaceRecordDetail {
            record: FaceRecord {
                id: rid.to_string(),
                user_id: Uuid::from_slice(&uid_b)
                    .map_err(|_| Status::InternalServerError)?
                    .to_string(),
                version,
                is_active: active != 0,
                created_at,
            },
            images: images
                .into_iter()
                .filter_map(|(iid, fp, h, ph, res, br, bl)| {
                    Some(FaceImage {
                        id: Uuid::from_slice(&iid).ok()?.to_string(),
                        face_record_id: rid.to_string(),
                        file_path: fp,
                        hash: h,
                        perceptual_hash: ph,
                        resolution: res,
                        brightness_score: br,
                        blur_score: bl,
                    })
                })
                .collect(),
            audits: audits
                .into_iter()
                .filter_map(|(aid, action, pb, ts)| {
                    Some(FaceAudit {
                        id: Uuid::from_slice(&aid).ok()?.to_string(),
                        face_record_id: rid.to_string(),
                        action,
                        performed_by: Uuid::from_slice(&pb).ok()?.to_string(),
                        created_at: ts,
                    })
                })
                .collect(),
            liveness: liveness
                .into_iter()
                .filter_map(|(cid, chal, passed, notes, pb, ts)| {
                    Some(FaceLivenessChallenge {
                        id: Uuid::from_slice(&cid).ok()?.to_string(),
                        face_record_id: rid.to_string(),
                        challenge: chal,
                        passed: passed != 0,
                        notes,
                        performed_by: Uuid::from_slice(&pb).ok()?.to_string(),
                        created_at: ts,
                    })
                })
                .collect(),
        });
    }

    Ok(Json(out))
}
