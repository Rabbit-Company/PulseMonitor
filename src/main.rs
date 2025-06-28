use crate::heartbeat::send_heartbeat;
use crate::services::mysql::is_mysql_online;
use crate::services::postgresql::is_postgresql_online;
use crate::services::redis::is_redis_online;
use clap::Parser;
use comfy_table::{presets::UTF8_FULL, Cell, Color, Table};
use inline_colorization::{color_blue, color_reset, color_white};
use services::{
	http::is_http_online, icmp::is_icmp_online, imap::is_imap_online, mssql::is_mssql_online,
	smtp::is_smtp_online, tcp::is_tcp_online, udp::is_udp_online, ws::is_ws_online,
};
use std::fs;
use std::time::Instant;
use tokio::time::{sleep, Duration};
use tracing::{error, info, Level};
use tracing_subscriber::EnvFilter;
use utils::{Config, VERSION};

mod heartbeat;
mod utils;
mod services {
	pub mod http;
	pub mod icmp;
	pub mod imap;
	pub mod mssql;
	pub mod mysql;
	pub mod postgresql;
	pub mod redis;
	pub mod smtp;
	pub mod tcp;
	pub mod udp;
	pub mod ws;
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Path to config.toml file
	#[arg(short, long, default_value_t = String::from("config.toml"))]
	config: String,
}

fn round_to_3_decimals(dec: f64) -> f64 {
	(dec * 1000.0).round() / 1000.0
}

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::from_default_env()
				.add_directive(Level::INFO.into())
				.add_directive("tiberius=off".parse().unwrap())
				.add_directive("tokio_util=off".parse().unwrap()),
		)
		.init();

	rustls::crypto::ring::default_provider()
		.install_default()
		.expect("Failed to install rustls crypto provider");

	let args: Args = Args::parse();
	let toml_string = fs::read_to_string(args.config).expect("Failed to read config file");
	let config: Config = toml::from_str(&toml_string).expect("Failed to parse TOML from config file");

	println!("{color_blue}PulseMonitor {}{color_reset}\n", VERSION);

	let mut rows: Vec<Vec<Cell>> = Vec::new();

	for monitor in config.monitors {
		if !monitor.enabled {
			continue;
		}

		let interval = monitor.interval;
		let cloned_monitor = monitor.clone();

		let service_type = if monitor.http.is_some() {
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
		} else {
			"None"
		};

		rows.push(vec![
			Cell::new(service_type).fg(Color::Blue),
			Cell::new(format!("{}s", monitor.interval)).fg(Color::DarkGrey),
			Cell::new(monitor.name).fg(Color::Green),
		]);

		tokio::spawn(async move {
			let mut interval_timer = tokio::time::interval(Duration::from_secs(interval));
			// First tick happens immediately, skip it to avoid double execution
			interval_timer.tick().await;

			loop {
				interval_timer.tick().await; // Wait for the next scheduled interval

				let start_time = Instant::now();

				let result = if cloned_monitor.http.is_some() {
					is_http_online(&cloned_monitor).await
				} else if cloned_monitor.ws.is_some() {
					is_ws_online(&cloned_monitor).await
				} else if cloned_monitor.tcp.is_some() {
					is_tcp_online(&cloned_monitor).await
				} else if cloned_monitor.udp.is_some() {
					is_udp_online(&cloned_monitor).await
				} else if cloned_monitor.icmp.is_some() {
					is_icmp_online(&cloned_monitor).await
				} else if cloned_monitor.smtp.is_some() {
					is_smtp_online(&cloned_monitor).await
				} else if cloned_monitor.imap.is_some() {
					is_imap_online(&cloned_monitor).await
				} else if cloned_monitor.mysql.is_some() {
					is_mysql_online(&cloned_monitor).await
				} else if cloned_monitor.mssql.is_some() {
					is_mssql_online(&cloned_monitor).await
				} else if cloned_monitor.postgresql.is_some() {
					is_postgresql_online(&cloned_monitor).await
				} else if cloned_monitor.redis.is_some() {
					is_redis_online(&cloned_monitor).await
				} else {
					Ok(None)
				};

				let latency_ms = match &result {
					Ok(Some(latency)) => round_to_3_decimals(*latency),
					_ => round_to_3_decimals(start_time.elapsed().as_secs_f64() * 1000.0),
				};

				match &result {
					Ok(_) => {
						if cloned_monitor.debug.unwrap_or(false) {
							info!(
								"Monitor '{}' succeed ({}ms)",
								cloned_monitor.name, latency_ms
							);
						}
						// Send heartbeat for every successful check
						if let Err(e) = send_heartbeat(&cloned_monitor, latency_ms).await {
							error!("Failed to send heartbeat for '{}': {}", cloned_monitor.name, e);
						}
					}
					Err(err) => {
						if cloned_monitor.debug.unwrap_or(false) {
							error!("Monitor '{}' failed: {}", cloned_monitor.name, err);
						}
						// TODO: Add option to send a 'down' heartbeat
					}
				}
			}
		});

		sleep(Duration::from_secs(1)).await;
	}

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

	loop {
		sleep(Duration::from_secs(3600)).await;
	}
}
