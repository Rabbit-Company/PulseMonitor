use clap::Parser;
use std::fs;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{Level, error, info, warn};
use tracing_subscriber::EnvFilter;
use utils::{Config, VERSION};

mod heartbeat;
mod monitor_runner;
mod pulse_queue;
mod utils;
mod ws_client;
mod services {
	pub mod http;
	pub mod icmp;
	pub mod imap;
	pub mod minecraft;
	pub mod mssql;
	pub mod mysql;
	pub mod postgresql;
	pub mod redis;
	pub mod smtp;
	pub mod snmp;
	pub mod tcp;
	pub mod udp;
	pub mod ws;
}

use monitor_runner::MonitorRunner;
use ws_client::WsClient;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Path to config.toml file (optional if using PULSE_SERVER_URL)
	#[arg(short, long, default_value_t = String::from("config.toml"))]
	config: String,
}

/// Configuration mode
enum ConfigMode {
	/// Use local config.toml file
	File(Config),
	/// Use WebSocket connection to UptimeMonitor-Server
	WebSocket { server_url: String, token: String },
}

fn load_env_config() -> Option<(String, String)> {
	// Try to load from .env file
	let _ = dotenvy::dotenv();

	let server_url = std::env::var("PULSE_SERVER_URL").ok();
	let token = std::env::var("PULSE_TOKEN").ok();

	match (server_url, token) {
		(Some(url), Some(tok)) if !url.is_empty() && !tok.is_empty() => Some((url, tok)),
		_ => None,
	}
}

fn determine_config_mode(args: &Args) -> Result<ConfigMode, Box<dyn std::error::Error>> {
	// First, check for environment variables
	if let Some((server_url, token)) = load_env_config() {
		info!("Using WebSocket mode with server: {}", server_url);
		return Ok(ConfigMode::WebSocket { server_url, token });
	}

	// Fall back to config file
	let config_path = &args.config;

	if fs::metadata(config_path).is_ok() {
		let toml_string = fs::read_to_string(config_path)?;
		let config: Config = toml::from_str(&toml_string)?;
		info!("Using config file: {}", config_path);
		return Ok(ConfigMode::File(config));
	}

	Err(
		format!(
			"No configuration found. Either set PULSE_SERVER_URL and PULSE_TOKEN environment variables, \
		or provide a config.toml file at '{}'",
			config_path
		)
		.into(),
	)
}

async fn run_file_mode(config: Config) {
	let runner = MonitorRunner::new();
	runner.start_monitors(&config).await;

	loop {
		sleep(Duration::from_secs(3600)).await;
	}
}

async fn run_websocket_mode(server_url: String, token: String) {
	let client = Arc::new(WsClient::new(&server_url, &token));

	// Get the pulse sender before starting the client
	let pulse_sender = client.get_pulse_sender();

	let mut config_rx = client.start().await;

	// Create runner with WebSocket pulse sender
	let runner = MonitorRunner::with_websocket(server_url, pulse_sender);

	info!("Waiting for configuration from server...");

	// Wait for configuration updates
	while let Some(config) = config_rx.recv().await {
		info!(
			"Applying new configuration with {} monitors",
			config.monitors.len()
		);

		// Update monitors with new config
		runner.update_monitors(&config).await;
	}

	// This should only happen if the channel is closed
	error!("Configuration channel closed unexpectedly");
}

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::from_default_env()
				.add_directive(Level::INFO.into())
				.add_directive("tiberius=off".parse().unwrap())
				.add_directive("tokio_util=off".parse().unwrap())
				.add_directive("elytra_ping=off".parse().unwrap()),
		)
		.init();

	rustls::crypto::ring::default_provider()
		.install_default()
		.expect("Failed to install rustls crypto provider");

	let args: Args = Args::parse();

	info!("PulseMonitor {}", VERSION);

	match determine_config_mode(&args) {
		Ok(ConfigMode::File(config)) => {
			info!("Mode: Local config file");
			run_file_mode(config).await;
		}
		Ok(ConfigMode::WebSocket { server_url, token }) => {
			info!("Mode: WebSocket ({})", server_url);
			run_websocket_mode(server_url, token).await;
		}
		Err(e) => {
			error!("Configuration error: {}", e);
			warn!(
				"To use WebSocket mode, set these environment variables:\n\
				  PULSE_SERVER_URL=http://localhost:3000\n\
				  PULSE_TOKEN=your_token_here\n\
				\n\
				Or create a config.toml file."
			);
			std::process::exit(1);
		}
	}
}
