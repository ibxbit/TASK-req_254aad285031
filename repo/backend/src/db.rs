use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

pub async fn init_pool(url: &str) -> anyhow::Result<MySqlPool> {
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(url)
        .await?;
    Ok(pool)
}
