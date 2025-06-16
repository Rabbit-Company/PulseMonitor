use crate::utils::Monitor;
use reqwest::Client;
use std::error::Error;
use std::sync::Arc;

pub async fn send_heartbeat(monitor: &Monitor, latency_ms: f64) -> Result<(), Box<dyn Error>> {
	let monitor = Arc::new(monitor.clone());

	let client = Client::new();

	let url = monitor
		.heartbeat
		.url
		.replace("{latency}", &latency_ms.to_string());

	let mut request = match monitor.heartbeat.method.to_uppercase().as_str() {
		"GET" => client.get(&url),
		"POST" => client.post(&url),
		"HEAD" => client.head(&url),
		_ => return Err(format!("Unsupported HTTP method: {}", monitor.heartbeat.method).into()),
	};

	if let Some(headers) = &monitor.heartbeat.headers {
		for header in headers {
			for (key, value) in header {
				let value_with_templates = value.replace("{latency}", &latency_ms.to_string());
				request = request.header(key, value_with_templates);
			}
		}
	}

	let response = request.send().await?;

	if response.status().is_success() {
		Ok(())
	} else {
		Err(format!("Request failed with status: {}", response.status()).into())
	}
}
