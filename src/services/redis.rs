use std::error::Error;

use crate::utils::Monitor;

pub async fn is_redis_online(monitor: &Monitor) -> Result<(), Box<dyn Error>> {
	let client = redis::Client::open(monitor.redis.as_ref().unwrap().url.clone())?;
	let mut con = client.get_connection()?;

	let _: () = redis::cmd("PING").query(&mut con)?;

	Ok(())
}