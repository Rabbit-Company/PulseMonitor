use crate::utils::Monitor;
use reqwest::Client;
use std::error::Error;
use std::sync::Arc;

pub async fn send_heartbeat(monitor: &Monitor) -> Result<(), Box<dyn Error>>{
	let monitor = Arc::new(monitor.clone());

	let client = Client::new();

	let request = match monitor.heartbeat.method.to_uppercase().as_str() {
		"GET" => client.get(&monitor.heartbeat.url),
		"POST" => client.post(&monitor.heartbeat.url),
		"HEAD" => client.head(&monitor.heartbeat.url),
		_ => return Err(format!("Unsupported HTTP method: {}", monitor.heartbeat.method).into())
	};

	let response = request.send().await?;

	if response.status().is_success() {
		Ok(())
	} else {
		Err(format!("Request failed with status: {}", response.status()).into())
	}
}