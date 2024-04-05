use std::fs;
use clap::Parser;
use utils::Config;
use inline_colorization::{color_blue,color_cyan,color_black,color_green,color_red};

mod utils;

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

		let mut colored_name = color_green;
		if !monitor.enabled { colored_name = color_red; }

		if let Some(mysql) = monitor.mysql {
			println!("{color_cyan}  - {colored_name}{} {color_black}({}s) - {color_blue}MySQL", monitor.name, monitor.execute_every);
		}else if let Some(redis) = monitor.redis{
			println!("{color_cyan}  - {colored_name}{} {color_black}({}s) - {color_blue}Redis", monitor.name, monitor.execute_every);
		}
	}

}
