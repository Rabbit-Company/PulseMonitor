use std::error::Error;
use tokio::time::Duration;

use crate::utils::Monitor;

pub async fn is_redis_online(
	monitor: &Monitor,
) -> Result<Option<f64>, Box<dyn Error + Send + Sync>> {
	let redis_config = monitor
		.redis
		.as_ref()
		.ok_or("Redis configuration missing")?;

	let timeout = Duration::from_secs(redis_config.timeout.unwrap_or(3));
	let client = redis::Client::open(redis_config.url.as_str())?;

	tokio::time::timeout(timeout, async {
		let mut conn = client.get_multiplexed_async_connection().await?;
		let _: String = redis::cmd("PING").query_async(&mut conn).await?;
		Ok(None)
	})
	.await
	.map_err(|_| "Redis connection timeout".into())
	.and_then(|result| result)
}
