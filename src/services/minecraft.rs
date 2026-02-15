use std::error::Error;
use std::time::Duration;

use crate::utils::{CheckResult, Monitor};

pub async fn is_minecraft_java_online(
	monitor: &Monitor,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let mc = monitor
		.minecraft_java
		.as_ref()
		.ok_or("Monitor does not contain Minecraft Java configuration")?;

	let timeout = Duration::from_secs(mc.timeout.unwrap_or(3));
	let addr = (mc.host.clone(), mc.port.unwrap_or(25565));

	let (info, latency) = elytra_ping::ping_or_timeout(addr, timeout).await.map_err(
		|e| -> Box<dyn Error + Send + Sync> { format!("Minecraft Java ping failed: {}", e).into() },
	)?;

	Ok(CheckResult {
		latency: Some(latency.as_secs_f64() * 1000.0),
		custom1: info.players.map(|p| p.online as f64),
		custom2: None,
		custom3: None,
	})
}

pub async fn is_minecraft_bedrock_online(
	monitor: &Monitor,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let mc = monitor
		.minecraft_bedrock
		.as_ref()
		.ok_or("Monitor does not contain Minecraft Bedrock configuration")?;

	let timeout = Duration::from_secs(mc.timeout.unwrap_or(3));
	let addr = (mc.host.clone(), mc.port.unwrap_or(19132));

	let (info, latency) = elytra_ping::bedrock::ping(addr, timeout, 1).await.map_err(
		|e| -> Box<dyn Error + Send + Sync> { format!("Minecraft Bedrock ping failed: {}", e).into() },
	)?;

	Ok(CheckResult {
		latency: Some(latency.as_secs_f64() * 1000.0),
		custom1: Some(info.online_players as f64),
		custom2: None,
		custom3: None,
	})
}
