use crate::utils::{Monitor, VERSION};
use base64::{engine::general_purpose, Engine};
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use reqwest::Client;
use std::error::Error;
use std::sync::Arc;

pub async fn send_heartbeat(monitor: &Monitor) -> Result<(), Box<dyn Error>>{
	let monitor = Arc::new(monitor.clone());

	let client = Client::new();

	let mut request = match monitor.heartbeat.method.to_uppercase().as_str() {
		"GET" => client.get(&monitor.heartbeat.url),
		"POST" => client.post(&monitor.heartbeat.url),
		"HEAD" => client.head(&monitor.heartbeat.url),
		_ => return Err(format!("Unsupported HTTP method: {}", monitor.heartbeat.method).into())
	}
	.header(USER_AGENT, format!("PulseMonitor {}", VERSION));

	if let Some(token) = &monitor.heartbeat.bearer_token {
		request = request.header(AUTHORIZATION, format!("Bearer {}", token));
	} else if let (Some(username), Some(password)) = (&monitor.heartbeat.username, &monitor.heartbeat.password) {
		let auth_value = format!("{}:{}", username, password);
		let encoded = general_purpose::STANDARD.encode(auth_value);
		request = request.header(AUTHORIZATION, format!("Basic {}", encoded));
	}

	let response = request.send().await?;

	if response.status().is_success() {
		Ok(())
	} else {
		Err(format!("Request failed with status: {}", response.status()).into())
	}
}