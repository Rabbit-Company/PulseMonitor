use std::error::Error;
use std::time::Duration;

use redis::AsyncConnectionConfig;

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

	let config = AsyncConnectionConfig::new()
		.set_connection_timeout(Some(timeout))
		.set_response_timeout(Some(timeout));

	let mut conn = client
		.get_multiplexed_async_connection_with_config(&config)
		.await?;

	let _: String = redis::cmd("PING").query_async(&mut conn).await?;

	Ok(None)
}
