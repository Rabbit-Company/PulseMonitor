use crate::heartbeat::send_heartbeat;
use crate::services::{
	http::is_http_online,
	icmp::is_icmp_online,
	imap::is_imap_online,
	minecraft::{is_minecraft_bedrock_online, is_minecraft_java_online},
	mssql::is_mssql_online,
	mysql::is_mysql_online,
	postgresql::is_postgresql_online,
	redis::is_redis_online,
	smtp::is_smtp_online,
	tcp::is_tcp_online,
	udp::is_udp_online,
	ws::is_ws_online,
};
use crate::utils::{CheckResult, Config, Monitor, PushMessage};
use chrono::Utc;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use inline_colorization::{color_reset, color_white};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};
use tracing::{error, info};

fn round_to_3_decimals(dec: f64) -> f64 {
	(dec * 1000.0).round() / 1000.0
}

fn get_service_type(monitor: &Monitor) -> &'static str {
	if monitor.http.is_some() {
		"HTTP"
	} else if monitor.ws.is_some() {
		"WS"
	} else if monitor.tcp.is_some() {
		"TCP"
	} else if monitor.udp.is_some() {
		"UDP"
	} else if monitor.icmp.is_some() {
		"ICMP"
	} else if monitor.smtp.is_some() {
		"SMTP"
	} else if monitor.imap.is_some() {
		"IMAP"
	} else if monitor.mysql.is_some() {
		"MySQL"
	} else if monitor.mssql.is_some() {
		"MSSQL"
	} else if monitor.postgresql.is_some() {
		"PostgreSQL"
	} else if monitor.redis.is_some() {
		"Redis"
	} else if monitor.minecraft_java.is_some() {
		"MC-Java"
	} else if monitor.minecraft_bedrock.is_some() {
		"MC-Bedrock"
	} else {
		"None"
	}
}

/// Type alias for the pulse sender
pub type PulseSender = Arc<RwLock<Option<mpsc::Sender<PushMessage>>>>;

/// Manages running monitor tasks
pub struct MonitorRunner {
	/// Map of monitor token/key to its cancel channel
	running_monitors: Arc<RwLock<HashMap<String, mpsc::Sender<()>>>>,
	/// Server URL for WebSocket mode (used as HTTP fallback)
	server_url: Option<String>,
	/// WebSocket pulse sender (shared with WsClient)
	pulse_sender: Option<PulseSender>,
}

impl MonitorRunner {
	/// Create a new MonitorRunner for file mode (no server_url needed)
	pub fn new() -> Self {
		MonitorRunner {
			running_monitors: Arc::new(RwLock::new(HashMap::new())),
			server_url: None,
			pulse_sender: None,
		}
	}

	/// Create a new MonitorRunner for WebSocket mode with server_url and pulse sender
	pub fn with_websocket(server_url: String, pulse_sender: PulseSender) -> Self {
		MonitorRunner {
			running_monitors: Arc::new(RwLock::new(HashMap::new())),
			server_url: Some(server_url),
			pulse_sender: Some(pulse_sender),
		}
	}

	/// Start monitors from config, returns handles to spawned tasks
	pub async fn start_monitors(&self, config: &Config) -> Vec<JoinHandle<()>> {
		let mut handles = Vec::new();
		let mut rows: Vec<Vec<Cell>> = Vec::new();

		for monitor in &config.monitors {
			if !monitor.enabled {
				continue;
			}

			let (cancel_tx, cancel_rx) = mpsc::channel::<()>(1);
			let monitor_clone = monitor.clone();
			let server_url_clone = self.server_url.clone();
			let pulse_sender_clone = self.pulse_sender.clone();
			let service_type = get_service_type(monitor);

			rows.push(vec![
				Cell::new(service_type).fg(Color::Blue),
				Cell::new(format!("{}s", monitor.interval)).fg(Color::DarkGrey),
				Cell::new(&monitor.name).fg(Color::Green),
			]);

			// Store the cancel channel using token as unique key (fallback to name if no token)
			let monitor_key = monitor
				.token
				.clone()
				.unwrap_or_else(|| monitor.name.clone());
			{
				let mut running = self.running_monitors.write().await;
				running.insert(monitor_key, cancel_tx);
			}

			// Spawn the monitor task
			let handle = tokio::spawn(async move {
				run_monitor(
					monitor_clone,
					server_url_clone,
					pulse_sender_clone,
					cancel_rx,
				)
				.await;
			});

			handles.push(handle);

			// Small delay between starting monitors
			sleep(Duration::from_millis(100)).await;
		}

		// Print the table
		if !rows.is_empty() {
			let mut table = Table::new();
			table
				.load_preset(UTF8_FULL)
				.set_style(comfy_table::TableComponent::HeaderLines, '─')
				.set_style(comfy_table::TableComponent::MiddleHeaderIntersections, '┼')
				.set_style(comfy_table::TableComponent::RightHeaderIntersection, '┤')
				.set_style(comfy_table::TableComponent::LeftHeaderIntersection, '├')
				.set_style(comfy_table::TableComponent::HorizontalLines, '─')
				.set_style(comfy_table::TableComponent::VerticalLines, '│')
				.set_header(vec!["Service", "Interval (s)", "Name"])
				.add_rows(rows);

			println!("{color_white}{}{color_reset}\n", table);
		}

		handles
	}

	/// Stop all running monitors
	pub async fn stop_all(&self) {
		let mut running = self.running_monitors.write().await;
		for (key, cancel_tx) in running.drain() {
			info!("Stopping monitor: {}", key);
			let _ = cancel_tx.send(()).await;
		}
	}

	/// Update monitors with new config
	/// This stops all current monitors and starts new ones
	pub async fn update_monitors(&self, config: &Config) -> Vec<JoinHandle<()>> {
		info!("Updating monitors with new configuration...");

		// Stop all existing monitors
		self.stop_all().await;

		// Give tasks time to clean up
		sleep(Duration::from_millis(500)).await;

		// Start new monitors
		self.start_monitors(config).await
	}
}

async fn run_monitor(
	monitor: Monitor,
	server_url: Option<String>,
	pulse_sender: Option<PulseSender>,
	mut cancel_rx: mpsc::Receiver<()>,
) {
	let mut interval_timer = tokio::time::interval(Duration::from_secs(monitor.interval));
	// First tick happens immediately, skip it to avoid double execution
	//interval_timer.tick().await;

	loop {
		tokio::select! {
			_ = cancel_rx.recv() => {
				info!("Monitor '{}' received cancel signal", monitor.name);
				break;
			}
			_ = interval_timer.tick() => {
				run_single_check(&monitor, server_url.as_deref(), pulse_sender.as_ref()).await;
			}
		}
	}
}

async fn run_single_check(
	monitor: &Monitor,
	server_url: Option<&str>,
	pulse_sender: Option<&PulseSender>,
) {
	let start_check_time = Utc::now();
	let start_time = Instant::now();

	let result: Result<CheckResult, Box<dyn std::error::Error + Send + Sync>> =
		if monitor.http.is_some() {
			is_http_online(monitor).await
		} else if monitor.ws.is_some() {
			is_ws_online(monitor).await
		} else if monitor.tcp.is_some() {
			is_tcp_online(monitor).await
		} else if monitor.udp.is_some() {
			is_udp_online(monitor).await
		} else if monitor.icmp.is_some() {
			is_icmp_online(monitor).await
		} else if monitor.smtp.is_some() {
			is_smtp_online(monitor).await
		} else if monitor.imap.is_some() {
			is_imap_online(monitor).await
		} else if monitor.mysql.is_some() {
			is_mysql_online(monitor).await
		} else if monitor.mssql.is_some() {
			is_mssql_online(monitor).await
		} else if monitor.postgresql.is_some() {
			is_postgresql_online(monitor).await
		} else if monitor.redis.is_some() {
			is_redis_online(monitor).await
		} else if monitor.minecraft_java.is_some() {
			is_minecraft_java_online(monitor).await
		} else if monitor.minecraft_bedrock.is_some() {
			is_minecraft_bedrock_online(monitor).await
		} else {
			Ok(CheckResult::from_latency(None))
		};

	let end_check_time = Utc::now();

	match &result {
		Ok(check_result) => {
			let latency_ms = match check_result.latency {
				Some(latency) => round_to_3_decimals(latency),
				None => round_to_3_decimals(start_time.elapsed().as_secs_f64() * 1000.0),
			};

			if monitor.debug.unwrap_or(false) {
				info!("Monitor '{}' succeed ({}ms)", monitor.name, latency_ms);
			}

			// Send heartbeat for every successful check
			if let Err(e) = send_heartbeat(
				monitor,
				server_url,
				pulse_sender,
				start_check_time,
				end_check_time,
				latency_ms,
				check_result,
			)
			.await
			{
				error!("Failed to send heartbeat for '{}': {}", monitor.name, e);
			}
		}
		Err(err) => {
			if monitor.debug.unwrap_or(false) {
				error!("Monitor '{}' failed: {}", monitor.name, err);
			}
			// TODO: Add option to send a 'down' heartbeat
		}
	}
}
