use std::error::Error;
use sqlx::mysql::MySqlPoolOptions;
use tokio::time::Duration;

use crate::utils::Monitor;

pub async fn is_mysql_online(monitor: &Monitor) -> Result<(), Box<dyn Error>> {
	let pool = MySqlPoolOptions::new()
	.acquire_timeout(Duration::from_secs(3))
	.connect(&monitor.mysql.as_ref().unwrap().url).await?;

	let _ = sqlx::query("SELECT 1").execute(&pool).await?;
	Ok(())
}