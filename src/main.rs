use std::fs;
use clap::Parser;
use utils::Config;
use std::thread;
use crate::services::mysql::is_mysql_online;
use crate::services::postgresql::is_postgresql_online;
use crate::services::redis::is_redis_online;
use crate::heartbeat::send_heartbeat;
use tokio::time::{sleep, Duration};
use inline_colorization::{color_blue,color_cyan,color_black,color_green};

mod utils;
mod heartbeat;
mod services {
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

	println!("{color_blue}Monitors:");
	for monitor in config.monitors {
		if !monitor.enabled { continue; }

		if monitor.mysql.is_some(){
			thread::spawn(move || {
				println!("{color_cyan}  - {color_green}{} {color_black}({}s) - {color_blue}MySQL", monitor.name, monitor.execute_every);
				loop {
					tokio::runtime::Runtime::new().unwrap().block_on(async {
						let res = is_mysql_online(&monitor).await;
						if res.is_ok(){
							send_heartbeat(&monitor).await.ok();
						}
						sleep(Duration::from_secs(monitor.execute_every)).await;
					});
				}
			});
		}else if monitor.postgresql.is_some(){
			thread::spawn(move || {
				println!("{color_cyan}  - {color_green}{} {color_black}({}s) - {color_blue}PostgreSQL", monitor.name, monitor.execute_every);
				loop {
					tokio::runtime::Runtime::new().unwrap().block_on(async {
						let res = is_postgresql_online(&monitor).await;
						if res.is_ok(){
							send_heartbeat(&monitor).await.ok();
						}
						sleep(Duration::from_secs(monitor.execute_every)).await;
					});
				}
			});
		} else if monitor.redis.is_some(){
			thread::spawn(move || {
				println!("{color_cyan}  - {color_green}{} {color_black}({}s) - {color_blue}Redis", monitor.name, monitor.execute_every);
				loop {
					tokio::runtime::Runtime::new().unwrap().block_on(async {
						let res = is_redis_online(&monitor).await;
						if res.is_ok(){
							send_heartbeat(&monitor).await.ok();
						}
						sleep(Duration::from_secs(monitor.execute_every)).await;
					});
				}
			});
		}

		sleep(Duration::from_secs(1)).await;
	}

	loop {
		sleep(Duration::from_secs(1)).await;
	}

}
