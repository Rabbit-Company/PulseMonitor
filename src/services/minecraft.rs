use std::error::Error;
use std::time::Duration;

use crate::utils::{CheckResult, Monitor};

pub const JAVA_DEFAULT_PORT: u16 = 25565;
pub const BEDROCK_DEFAULT_PORT: u16 = 19132;

/// Result of a Minecraft server list ping
pub struct MinecraftStatus {
	pub latency: Duration,
	pub players_online: Option<u32>,
	pub players_maximum: Option<u32>,
}

pub async fn ping_java(
	host: &str,
	port: u16,
	timeout: Duration,
) -> Result<MinecraftStatus, Box<dyn Error + Send + Sync>> {
	let (info, latency) = elytra_ping::ping_or_timeout((host.to_string(), port), timeout)
		.await
		.map_err(|e| -> Box<dyn Error + Send + Sync> {
			format!("Minecraft Java ping failed: {}", e).into()
		})?;

	Ok(MinecraftStatus {
		latency,
		players_online: info.players.as_ref().map(|players| players.online),
		players_maximum: info.players.as_ref().map(|players| players.max),
	})
}

pub async fn ping_bedrock(
	host: &str,
	port: u16,
	timeout: Duration,
) -> Result<MinecraftStatus, Box<dyn Error + Send + Sync>> {
	let (info, latency) = elytra_ping::bedrock::ping((host.to_string(), port), timeout, 1)
		.await
		.map_err(|e| -> Box<dyn Error + Send + Sync> {
			format!("Minecraft Bedrock ping failed: {}", e).into()
		})?;

	Ok(MinecraftStatus {
		latency,
		players_online: Some(info.online_players),
		players_maximum: Some(info.max_players),
	})
}

fn to_check_result(status: &MinecraftStatus) -> CheckResult {
	let mut result = CheckResult::new();
	result.set("latency", status.latency.as_secs_f64() * 1000.0);
	if let Some(online) = status.players_online {
		result.set("custom1", online as f64);
		result.set("playerCount", online as f64);
	}
	result
}

pub async fn is_minecraft_java_online(
	monitor: &Monitor,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let mc = monitor
		.minecraft_java
		.as_ref()
		.ok_or("Monitor does not contain Minecraft Java configuration")?;

	let timeout = Duration::from_secs(mc.timeout.unwrap_or(3));
	let status = ping_java(&mc.host, mc.port.unwrap_or(JAVA_DEFAULT_PORT), timeout).await?;

	Ok(to_check_result(&status))
}

pub async fn is_minecraft_bedrock_online(
	monitor: &Monitor,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let mc = monitor
		.minecraft_bedrock
		.as_ref()
		.ok_or("Monitor does not contain Minecraft Bedrock configuration")?;

	let timeout = Duration::from_secs(mc.timeout.unwrap_or(3));
	let status = ping_bedrock(&mc.host, mc.port.unwrap_or(BEDROCK_DEFAULT_PORT), timeout).await?;

	Ok(to_check_result(&status))
}
