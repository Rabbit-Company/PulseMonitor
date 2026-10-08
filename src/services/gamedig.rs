use std::error::Error;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use gamedig::protocols::gamespy::GameSpyVersion;
use gamedig::protocols::quake::QuakeVersion;
use gamedig::protocols::types::{GatherToggle, Protocol};
use gamedig::protocols::valve::Engine;
use gamedig::{
	ExtraRequestSettings, GAMES, GDError, Game, TimeoutSettings,
	query_with_timeout_and_extra_settings,
};

use crate::services::minecraft::{
	BEDROCK_DEFAULT_PORT, JAVA_DEFAULT_PORT, MinecraftStatus, ping_bedrock, ping_java,
};
use crate::utils::{CheckResult, GamedigConfig, Monitor};

/// Generic protocols for game servers that do not have a GameDig game ID
pub const GAMEDIG_PROTOCOLS: [&str; 8] = [
	"valve", "gamespy1", "gamespy2", "gamespy3", "quake1", "quake2", "quake3", "unreal2",
];

fn generic_protocol(name: &str) -> Option<Protocol> {
	Some(match name {
		"valve" => Protocol::Valve(Engine::Source(None)),
		"gamespy1" => Protocol::Gamespy(GameSpyVersion::One),
		"gamespy2" => Protocol::Gamespy(GameSpyVersion::Two),
		"gamespy3" => Protocol::Gamespy(GameSpyVersion::Three),
		"quake1" => Protocol::Quake(QuakeVersion::One),
		"quake2" => Protocol::Quake(QuakeVersion::Two),
		"quake3" => Protocol::Quake(QuakeVersion::Three),
		"unreal2" => Protocol::Unreal2,
		_ => return None,
	})
}

/// Resolve the configured game ID or generic protocol into a GameDig game definition
fn resolve_game(config: &GamedigConfig) -> Result<Game, Box<dyn Error + Send + Sync>> {
	match (config.game.as_deref(), config.protocol.as_deref()) {
		(Some(_), Some(_)) => Err("GameDig: set either 'game' or 'protocol', not both".into()),
		(None, None) => Err("GameDig: either 'game' or 'protocol' is required".into()),
		(Some(id), None) => GAMES
			.get(id.to_lowercase().as_str())
			.cloned()
			.ok_or_else(|| format!("GameDig: unknown game '{}'", id).into()),
		(None, Some(name)) => {
			let protocol = generic_protocol(name.to_lowercase().as_str()).ok_or_else(|| {
				format!(
					"GameDig: unknown protocol '{}' (supported: {})",
					name,
					GAMEDIG_PROTOCOLS.join(", ")
				)
			})?;

			if config.port.is_none() {
				return Err("GameDig: 'port' is required when using 'protocol'".into());
			}

			Ok(Game {
				name: "Generic",
				default_port: 0,
				protocol,
				request_settings: ExtraRequestSettings::default(),
			})
		}
	}
}

async fn resolve_host(host: &str) -> Result<IpAddr, Box<dyn Error + Send + Sync>> {
	if let Ok(ip) = host.parse::<IpAddr>() {
		return Ok(ip);
	}

	let addresses: Vec<IpAddr> = tokio::net::lookup_host((host, 0))
		.await
		.map_err(|e| format!("GameDig: failed to resolve '{}': {}", host, e))?
		.map(|addr| addr.ip())
		.collect();

	// GameDig's UDP queries only work over IPv4, so prefer an IPv4 address when there is one
	addresses
		.iter()
		.find(|ip| ip.is_ipv4())
		.or(addresses.first())
		.copied()
		.ok_or_else(|| format!("GameDig: no addresses found for '{}'", host).into())
}

/// One-line description of a GameDig error (its Display output spans several lines)
fn describe_error(error: &GDError) -> String {
	match &error.source {
		Some(source) => format!("{:?} ({})", error.kind, source),
		None => format!("{:?}", error.kind),
	}
}

fn build_result(
	latency: Duration,
	players_online: Option<u32>,
	players_maximum: Option<u32>,
	players_bots: Option<u32>,
) -> CheckResult {
	let mut result = CheckResult::new();
	result.set("latency", latency.as_secs_f64() * 1000.0);
	if let Some(online) = players_online {
		result.set("custom1", online as f64);
		result.set("playerCount", online as f64);
	}
	if let Some(maximum) = players_maximum {
		result.set("custom2", maximum as f64);
		result.set("maxPlayers", maximum as f64);
	}
	if let Some(bots) = players_bots {
		result.set("custom3", bots as f64);
		result.set("botCount", bots as f64);
	}
	result
}

/// Modern Minecraft servers are queried with the same client as the dedicated Minecraft
/// monitors. GameDig's Java query waits for the server to close the connection, which
/// proxy networks (BungeeCord, Velocity) do not do, so it times out against them.
async fn query_minecraft(
	game_id: &str,
	config: &GamedigConfig,
	timeout: Duration,
) -> Option<Result<MinecraftStatus, Box<dyn Error + Send + Sync>>> {
	let java_port = config.port.unwrap_or(JAVA_DEFAULT_PORT);
	let bedrock_port = config.port.unwrap_or(BEDROCK_DEFAULT_PORT);

	match game_id {
		"minecraftjava" => Some(ping_java(&config.host, java_port, timeout).await),
		"minecraftbedrock" | "minecraftpocket" => {
			Some(ping_bedrock(&config.host, bedrock_port, timeout).await)
		}
		// Edition not specified: try Java first, then Bedrock
		"minecraft" => match ping_java(&config.host, java_port, timeout).await {
			Ok(status) => Some(Ok(status)),
			Err(java_error) => Some(
				ping_bedrock(&config.host, bedrock_port, timeout)
					.await
					.map_err(|bedrock_error| format!("{}; {}", java_error, bedrock_error).into()),
			),
		},
		_ => None,
	}
}

pub async fn is_gamedig_online(
	monitor: &Monitor,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let config = monitor
		.gamedig
		.as_ref()
		.ok_or("Monitor does not contain GameDig configuration")?;

	let game = resolve_game(config)?;
	let timeout = Duration::from_secs(config.timeout.unwrap_or(5).max(1));

	if let Some(game_id) = config.game.as_deref()
		&& let Some(status) = query_minecraft(&game_id.to_lowercase(), config, timeout).await
	{
		let status = status?;
		return Ok(build_result(
			status.latency,
			status.players_online,
			status.players_maximum,
			None,
		));
	}

	let timeout_settings = TimeoutSettings::new(Some(timeout), Some(timeout), Some(timeout), 0)
		.map_err(|e| format!("GameDig: invalid timeout: {}", e))?;

	let address = tokio::time::timeout(timeout, resolve_host(&config.host))
		.await
		.map_err(|_| format!("GameDig: resolving '{}' timed out", config.host))??;
	let port = config.port;

	let mut extra_settings = game.request_settings.clone();

	// Player counts come from the server info reply. The optional player list and rules
	// requests go unanswered on many servers, and waiting for them would exceed the timeout.
	extra_settings.gather_players = Some(GatherToggle::Skip);
	extra_settings.gather_rules = Some(GatherToggle::Skip);

	// A server that answers the query is up, even when it reports a different Steam app ID
	// than GameDig expects (mods, re-released games and some dedicated server builds do)
	extra_settings.check_app_id = Some(false);

	// Minecraft servers behind a proxy route on the hostname the client connected with
	if config.host.parse::<IpAddr>().is_err() {
		extra_settings.hostname = Some(config.host.clone());
	}

	// GameDig is a blocking library, so the query runs on the blocking thread pool
	let query = tokio::task::spawn_blocking(move || {
		let started = Instant::now();
		let response = query_with_timeout_and_extra_settings(
			&game,
			&address,
			port,
			Some(timeout_settings),
			Some(extra_settings),
		)
		.map_err(|e| format!("GameDig query failed: {}", describe_error(&e)))?;

		Ok::<_, String>((
			started.elapsed(),
			response.players_online(),
			response.players_maximum(),
			response.players_bots(),
		))
	});

	let (latency, players_online, players_maximum, players_bots) =
		tokio::time::timeout(timeout, query)
			.await
			.map_err(|_| "GameDig query timed out")?
			.map_err(|e| format!("GameDig query task failed: {}", e))??;

	Ok(build_result(
		latency,
		Some(players_online),
		Some(players_maximum),
		players_bots,
	))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn config(game: Option<&str>, protocol: Option<&str>, port: Option<u16>) -> GamedigConfig {
		GamedigConfig {
			game: game.map(String::from),
			protocol: protocol.map(String::from),
			host: "127.0.0.1".to_string(),
			port,
			timeout: Some(1),
		}
	}

	#[test]
	fn resolves_known_game_ids() {
		let game = resolve_game(&config(Some("valheim"), None, None)).unwrap();
		assert_eq!(game.name, "Valheim");
		assert_eq!(game.default_port, 2457);

		assert!(resolve_game(&config(Some("MinecraftJava"), None, None)).is_ok());
	}

	#[test]
	fn rejects_unknown_game_ids() {
		let err = resolve_game(&config(Some("not-a-game"), None, None)).unwrap_err();
		assert!(err.to_string().contains("unknown game 'not-a-game'"));
	}

	#[test]
	fn resolves_every_generic_protocol() {
		for name in GAMEDIG_PROTOCOLS {
			assert!(
				resolve_game(&config(None, Some(name), Some(27016))).is_ok(),
				"{}",
				name
			);
		}
	}

	#[test]
	fn generic_protocol_requires_a_port() {
		let err = resolve_game(&config(None, Some("valve"), None)).unwrap_err();
		assert!(err.to_string().contains("'port' is required"));
	}

	#[test]
	fn rejects_unknown_protocols() {
		let err = resolve_game(&config(None, Some("carrier-pigeon"), Some(1))).unwrap_err();
		assert!(err.to_string().contains("unknown protocol"));
	}

	#[test]
	fn requires_exactly_one_of_game_and_protocol() {
		assert!(resolve_game(&config(None, None, None)).is_err());
		assert!(resolve_game(&config(Some("rust"), Some("valve"), Some(1))).is_err());
	}

	#[test]
	fn parses_the_config_sent_by_the_server() {
		let monitor: Monitor = serde_json::from_str(
			r#"{
				"enabled": true,
				"name": "Space Engineers",
				"token": "tk",
				"interval": 10,
				"gamedig": { "protocol": "valve", "host": "se.example.com", "port": 27016, "timeout": 4 }
			}"#,
		)
		.unwrap();

		let config = monitor.gamedig.unwrap();
		assert_eq!(config.protocol.as_deref(), Some("valve"));
		assert_eq!(config.game, None);
		assert_eq!(config.host, "se.example.com");
		assert_eq!(config.port, Some(27016));
		assert_eq!(config.timeout, Some(4));
	}

	#[test]
	fn parses_a_toml_config_with_only_a_game_and_host() {
		let monitor: Monitor = toml::from_str(
			r#"
				enabled = true
				name = "Valheim"
				interval = 30

				[gamedig]
				game = "valheim"
				host = "game.example.com"
			"#,
		)
		.unwrap();

		let config = monitor.gamedig.unwrap();
		assert_eq!(config.game.as_deref(), Some("valheim"));
		assert_eq!(config.port, None);
		assert_eq!(config.timeout, None);
	}

	#[test]
	fn placeholders_are_empty_when_a_game_reports_no_bots() {
		let monitor = Monitor {
			gamedig: Some(config(Some("q3a"), None, None)),
			..Default::default()
		};
		let result = build_result(Duration::from_millis(20), Some(3), Some(16), None);
		let placeholders = crate::utils::resolve_custom_placeholders(&monitor, &result);
		let value = |name: &str| {
			let matches: Vec<_> = placeholders.iter().filter(|(key, _)| key == name).collect();
			assert_eq!(matches.len(), 1, "{} should be emitted exactly once", name);
			matches[0].1.clone()
		};

		assert_eq!(value("{playerCount}"), "3");
		assert_eq!(value("{maxPlayers}"), "16");
		assert_eq!(value("{botCount}"), "");
		assert_eq!(value("{custom3}"), "");
	}

	#[tokio::test]
	async fn resolves_ip_addresses_without_dns() {
		assert_eq!(
			resolve_host("127.0.0.1").await.unwrap(),
			"127.0.0.1".parse::<IpAddr>().unwrap()
		);
		assert_eq!(
			resolve_host("::1").await.unwrap(),
			"::1".parse::<IpAddr>().unwrap()
		);
	}

	#[tokio::test]
	async fn prefers_ipv4_when_resolving_hostnames() {
		assert!(resolve_host("localhost").await.unwrap().is_ipv4());
	}

	#[tokio::test]
	async fn fails_when_nothing_answers() {
		let monitor = Monitor {
			gamedig: Some(config(None, Some("valve"), Some(9))),
			..Default::default()
		};
		assert!(is_gamedig_online(&monitor).await.is_err());
	}
}
