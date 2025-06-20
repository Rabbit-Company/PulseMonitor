use std::error::Error;
use tokio::time::Duration;

use crate::utils::Monitor;

pub async fn is_redis_online(
	monitor: &Monitor,
) -> Result<Option<f64>, Box<dyn Error + Send + Sync>> {
	let redis = monitor
		.redis
		.as_ref()
		.ok_or("Monitor does not contain Redis configuration")?;

	let timeout_duration: Duration = Duration::from_secs(redis.timeout.unwrap_or(3));

	let client = redis::Client::open(redis.url.clone())?;

	let result = tokio::time::timeout(timeout_duration, async {
		let mut con = client.get_multiplexed_tokio_connection().await?;
		let _: () = redis::cmd("PING").query_async(&mut con).await?;
		Ok(())
	})
	.await;

	match result {
		Ok(Ok(_)) => Ok(None),
		Ok(Err(e)) => Err(e),
		Err(_) => Err(Box::new(std::io::Error::new(
			std::io::ErrorKind::TimedOut,
			"Redis connection timed out",
		))),
	}
}
