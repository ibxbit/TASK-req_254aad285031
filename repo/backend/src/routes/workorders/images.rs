use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{ReviewImage, Role};
use sqlx::MySqlPool;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::audit;
use crate::auth::guard::AuthUser;
use crate::config::StorageConfig;
use crate::face::content_hash_hex;
use crate::workorders::{MAX_IMAGES_PER_REVIEW, MAX_IMAGE_SIZE_BYTES};

#[derive(FromForm)]
pub struct ImageUploadForm<'r> {
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

#[post("/reviews/<id>/images", data = "<upload>")]
pub async fn upload(
    pool: &State<MySqlPool>,
    storage_cfg: &State<StorageConfig>,
    user: AuthUser,
    id: &str,
    upload: Form<ImageUploadForm<'_>>,
) -> Result<Json<ReviewImage>, Status> {
    let rid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    // Ownership: only the review author (or admin) may attach images.
    let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT user_id FROM reviews WHERE id = ?")
        .bind(&rid.as_bytes()[..])
        .fetch_optional(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    let Some((author_b,)) = row else {
        return Err(Status::NotFound);
    };
    let author = Uuid::from_slice(&author_b).map_err(|_| Status::InternalServerError)?;
    if author != user.id && user.role != Role::Administrator {
        return Err(Status::Forbidden);
    }

    // Max 5 images per review (DB has no count-check; enforce here).
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM review_images WHERE review_id = ?")
        .bind(&rid.as_bytes()[..])
        .fetch_one(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    if count.0 >= MAX_IMAGES_PER_REVIEW {
        return Err(Status::Conflict);
    }

    let mut form = upload.into_inner();

    let size = form.file.len();
    if size == 0 || size > MAX_IMAGE_SIZE_BYTES {
        return Err(Status::PayloadTooLarge);
    }

    let (mime, ext) = {
        let ct = form
            .file
            .content_type()
            .ok_or(Status::UnsupportedMediaType)?;
        if ct.is_jpeg() {
            ("image/jpeg", "jpg")
        } else if ct.is_png() {
            ("image/png", "png")
        } else {
            return Err(Status::UnsupportedMediaType);
        }
    };

    // Read bytes in full so we can compute a SHA-256 content_hash alongside
    // the stored file — enables integrity verification for any review image.
    let bytes = read_temp_file(&mut form.file)
        .await
        .map_err(|_| Status::InternalServerError)?;
    if bytes.is_empty() {
        return Err(Status::BadRequest);
    }
    let content_hex = content_hash_hex(&bytes);

    let img_id = Uuid::new_v4();
    let filename = format!("{}.{}", img_id, ext);
    let dir = std::path::PathBuf::from(&storage_cfg.review_images_dir);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let path = dir.join(&filename);
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let rel_path = path.to_string_lossy().to_string();
    let size_i32: i32 = bytes
        .len()
        .try_into()
        .map_err(|_| Status::PayloadTooLarge)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    sqlx::query(
        "INSERT INTO review_images \
         (id, review_id, file_path, size, content_type, content_hash) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&img_id.as_bytes()[..])
    .bind(&rid.as_bytes()[..])
    .bind(&rel_path)
    .bind(size_i32)
    .bind(mime)
    .bind(&content_hex)
    .execute(&mut *tx)
    .await
    .map_err(|_| Status::InternalServerError)?;

    let audit_payload = serde_json::json!({
        "action": "upload",
        "actor": user.id.to_string(),
        "review_id": rid.to_string(),
        "file_path": rel_path,
        "size": size_i32,
        "content_type": mime,
        "content_hash": content_hex,
    });
    audit::record_event_tx(&mut tx, "review_image", img_id, "upload", &audit_payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(ReviewImage {
        id: img_id.to_string(),
        review_id: rid.to_string(),
        file_path: rel_path,
        size: size_i32,
        content_type: mime.to_string(),
        content_hash: Some(content_hex),
    }))
}
