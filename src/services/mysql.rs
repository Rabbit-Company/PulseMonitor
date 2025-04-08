use std::error::Error;
use sqlx::mysql::MySqlPoolOptions;
use tokio::time::Duration;

use crate::utils::Monitor;

pub async fn is_mysql_online(monitor: &Monitor) -> Result<(), Box<dyn Error + Send + Sync>> {
	let mysql = monitor
		.mysql
		.as_ref()
		.ok_or("Monitor does not contain MYSQL configuration")?;

	let pool = MySqlPoolOptions::new()
		.acquire_timeout(Duration::from_secs(mysql.timeout.unwrap_or(3)))
		.connect(&mysql.url).await?;

	let _ = sqlx::query("SELECT 1").execute(&pool).await?;

	Ok(())
}