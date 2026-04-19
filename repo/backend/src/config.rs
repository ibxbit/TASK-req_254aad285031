use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EncryptionConfig {
    pub key_file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub review_images_dir: String,
    pub report_attachments_dir: String,
    pub face_images_dir: String,
}

fn default_tz_offset() -> i32 {
    0
}
fn default_grace_hours() -> i64 {
    72
}

#[derive(Debug, Clone, Deserialize)]
pub struct PolicyConfig {
    #[serde(default = "default_tz_offset")]
    pub local_timezone_offset_minutes: i32,
    #[serde(default = "default_grace_hours")]
    pub late_grace_hours: i64,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            local_timezone_offset_minutes: 0,
            late_grace_hours: 72,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub encryption: EncryptionConfig,
    pub storage: StorageConfig,
    #[serde(default)]
    pub policy: PolicyConfig,
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let text = fs::read_to_string(path)?;
        let cfg: AppConfig = toml::from_str(&text)?;
        Ok(cfg)
    }
}
