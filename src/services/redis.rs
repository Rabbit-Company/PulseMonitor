use std::error::Error;

use crate::utils::Monitor;

pub async fn is_redis_online(monitor: &Monitor) -> Result<(), Box<dyn Error + Send + Sync>> {
	let redis = monitor
		.redis
		.as_ref()
		.ok_or("Monitor does not contain Redis configuration")?;

	let client = redis::Client::open(redis.url.clone())?;
	let mut con = client.get_connection()?;

	let _: () = redis::cmd("PING").query(&mut con)?;

	Ok(())
}