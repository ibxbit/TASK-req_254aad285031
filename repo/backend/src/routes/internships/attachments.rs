use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{ReportAttachment, Role};
use sqlx::MySqlPool;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::audit;
use crate::auth::guard::AuthUser;
use crate::config::StorageConfig;
use crate::face::content_hash_hex;
use crate::logging;

#[derive(FromForm)]
pub struct AttachmentForm<'r> {
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

#[post("/reports/<id>/attachments", data = "<upload>")]
pub async fn upload(
    pool: &State<MySqlPool>,
    storage_cfg: &State<StorageConfig>,
    user: AuthUser,
    id: &str,
    upload: Form<AttachmentForm<'_>>,
) -> Result<Json<ReportAttachment>, Status> {
    let rid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    // Only the intern who authored the report (or an admin) may attach.
    let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT intern_id FROM reports WHERE id = ?")
        .bind(&rid.as_bytes()[..])
        .fetch_optional(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    let Some((author_b,)) = row else {
        return Err(Status::NotFound);
    };
    let author = Uuid::from_slice(&author_b).map_err(|_| Status::InternalServerError)?;
    if author != user.id && user.role != Role::Administrator {
        logging::permission_denied(
            &user.id.to_string(),
            user.role.as_str(),
            "POST /reports/<id>/attachments",
        );
        return Err(Status::Forbidden);
    }

    let mut form = upload.into_inner();
    if form.file.len() == 0 {
        return Err(Status::BadRequest);
    }

    // Read bytes in full so we can both compute a SHA-256 content hash
    // AND write them to disk under a known path. Integrity hash is stored
    // alongside the row so any later tampering can be detected.
    let bytes = read_temp_file(&mut form.file)
        .await
        .map_err(|_| Status::InternalServerError)?;
    if bytes.is_empty() {
        return Err(Status::BadRequest);
    }
    let content_hex = content_hash_hex(&bytes);
    let size_i64: i64 = bytes.len() as i64;

    let ext = form
        .file
        .name()
        .and_then(|n| n.rsplit_once('.').map(|(_, e)| e.to_string()))
        .unwrap_or_else(|| "bin".to_string());

    let att_id = Uuid::new_v4();
    let filename = format!("{}.{}", att_id, ext);
    let dir = std::path::PathBuf::from(&storage_cfg.report_attachments_dir);
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

    sqlx::query(
        "INSERT INTO report_attachments \
         (id, report_id, file_path, content_hash, size_bytes) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&att_id.as_bytes()[..])
    .bind(&rid.as_bytes()[..])
    .bind(&rel_path)
    .bind(&content_hex)
    .bind(size_i64)
    .execute(&mut *tx)
    .await
    .map_err(|_| Status::InternalServerError)?;

    let audit_payload = serde_json::json!({
        "action": "upload",
        "actor": user.id.to_string(),
        "report_id": rid.to_string(),
        "file_path": rel_path,
        "size_bytes": size_i64,
        "content_hash": content_hex,
    });
    audit::record_event_tx(
        &mut tx,
        "report_attachment",
        att_id,
        "upload",
        &audit_payload,
    )
    .await
    .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(ReportAttachment {
        id: att_id.to_string(),
        report_id: rid.to_string(),
        file_path: rel_path,
        content_hash: Some(content_hex),
        size_bytes: Some(size_i64),
    }))
}
