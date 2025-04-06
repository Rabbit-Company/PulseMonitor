use std::error::Error;
use sqlx::postgres::PgPoolOptions;
use tokio::time::Duration;

use crate::utils::Monitor;

pub async fn is_postgresql_online(monitor: &Monitor) -> Result<(), Box<dyn Error + Send + Sync>> {
	let pool = PgPoolOptions::new()
	.acquire_timeout(Duration::from_secs(3))
	.connect(&monitor.postgresql.as_ref().unwrap().url).await?;

	let _ = sqlx::query("SELECT 1").execute(&pool).await?;
	Ok(())
}