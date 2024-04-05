use std::fs;
use clap::Parser;
use toml::Value;

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
	let config: Value = toml::from_str(&toml_string).expect("Failed to parse TOML from config file");

	println!("{}", config);
}
