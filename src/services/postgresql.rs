use std::error::Error;
use sqlx::postgres::PgPoolOptions;
use tokio::time::Duration;

use crate::utils::Monitor;

pub async fn is_postgresql_online(monitor: &Monitor) -> Result<(), Box<dyn Error + Send + Sync>> {
	let postgresql = monitor
		.postgresql
		.as_ref()
		.ok_or("Monitor does not contain PostgreSQL configuration")?;

	let pool = PgPoolOptions::new()
		.acquire_timeout(Duration::from_secs(postgresql.timeout.unwrap_or(3)))
		.connect(&postgresql.url).await?;

	let _ = sqlx::query("SELECT 1").execute(&pool).await?;

	Ok(())
}