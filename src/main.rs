use std::fs;
use clap::Parser;
use comfy_table::{presets::UTF8_FULL, Cell, Color, Table};
use services::http::is_http_online;
use utils::{Config, VERSION};
use crate::services::mysql::is_mysql_online;
use crate::services::postgresql::is_postgresql_online;
use crate::services::redis::is_redis_online;
use crate::heartbeat::send_heartbeat;
use tokio::time::{sleep, Duration};
use inline_colorization::{color_blue,color_white};

mod utils;
mod heartbeat;
mod services {
	pub mod http;
	pub mod mysql;
	pub mod postgresql;
	pub mod redis;
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Path to config.toml file
	#[arg(short, long, default_value_t = String::from("config.toml"))]
	config: String
}

#[tokio::main]
async fn main() {
	let args: Args = Args::parse();
	let toml_string = fs::read_to_string(args.config).expect("Failed to read config file");
	let config: Config = toml::from_str(&toml_string).expect("Failed to parse TOML from config file");

	println!("{color_blue}PulseMonitor {}", VERSION);

	let mut rows: Vec<Vec<Cell>> = Vec::new();

	for monitor in config.monitors {
		if !monitor.enabled { continue; }

		let interval = monitor.execute_every;
		let cloned_monitor = monitor.clone();

		let service_type = if monitor.http.is_some() {
			"HTTP"
		} else if monitor.mysql.is_some() {
			"MySQL"
		} else if monitor.postgresql.is_some() {
			"PostgreSQL"
		} else if monitor.redis.is_some() {
			"Redis"
		} else {
			"None"
		};

		rows.push(vec![
			Cell::new(service_type).fg(Color::Blue),
			Cell::new(format!("{}s", monitor.execute_every)).fg(Color::DarkGrey),
			Cell::new(monitor.name).fg(Color::Green),
		]);

		tokio::spawn(async move {
			loop {
				let result = if cloned_monitor.http.is_some() {
					is_http_online(&cloned_monitor).await
				} else if cloned_monitor.mysql.is_some() {
					is_mysql_online(&cloned_monitor).await
				} else if cloned_monitor.postgresql.is_some() {
					is_postgresql_online(&cloned_monitor).await
				} else if cloned_monitor.redis.is_some() {
					is_redis_online(&cloned_monitor).await
				} else {
					Ok(())
				};

				if result.is_ok() {
					let _ = send_heartbeat(&cloned_monitor).await;
				}

				sleep(Duration::from_secs(interval)).await;
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

	println!("\n{color_white}{}", table);

	loop {
		sleep(Duration::from_secs(3600)).await;
	}

}
