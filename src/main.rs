use std::fs;
use clap::Parser;
use toml::Value;
use serde::Deserialize;
use inline_colorization::{color_blue,color_cyan,color_black,color_green,color_red};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Path to config.toml file
	#[arg(short, long, default_value_t = String::from("config.toml"))]
	config: String
}

#[derive(Debug, Deserialize)]
struct Monitor {
	enabled: bool,
	name: String,
	execute_every: i32,
	heartbeat: Heartbeat,
	#[serde(flatten)]
	specific: Specific,
}

#[derive(Debug, Deserialize)]
struct Heartbeat {
	method: String,
	url: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Specific {
	#[serde(rename = "mysql")]
	MySQL { url: String },
	#[serde(rename = "redis")]
	Redis { url: String },
}

#[tokio::main]
async fn main() {
	let args: Args = Args::parse();

	let toml_string = fs::read_to_string(args.config).expect("Failed to read config file");
	let monitors: Vec<Monitor> = toml::from_str(&toml_string).expect("Failed to parse TOML from config file");

	println!("{color_blue}Monitors:");
	for monitor in monitors {

		let mut colored_name = color_green;
		if !monitor.enabled { colored_name = color_red; }

		match monitor.specific {
			Specific::MySQL { url } => {
				println!("{color_cyan}  - {colored_name}{} {color_black}({}s) - {color_blue}MySQL", monitor.name, monitor.execute_every);
			}
			Specific::Redis { url } => {
				println!("{color_cyan}  - {colored_name}{} {color_black}({}s) - {color_blue}Redis", monitor.name, monitor.execute_every);
			}
		}
	}

}
