use std::error::Error;
use base64::{engine::general_purpose, Engine};
use reqwest::{header::{AUTHORIZATION, USER_AGENT}, Client};

use crate::utils::{Monitor, VERSION};

pub async fn is_http_online(monitor: &Monitor) -> Result<(), Box<dyn Error + Send + Sync>> {
	let http = monitor
		.http
		.as_ref()
		.ok_or("Monitor does not contain HTTP configuration")?;

	let client = Client::new();

	let mut request = match http.method.to_uppercase().as_str() {
		"GET" => client.get(&http.url),
		"POST" => client.post(&http.url),
		"HEAD" => client.head(&http.url),
		_ => return Err(format!("Unsupported HTTP method: {}", http.method).into())
	}
	.header(USER_AGENT, format!("PulseMonitor {}", VERSION));

	if let Some(token) = &http.bearer_token {
		request = request.header(AUTHORIZATION, format!("Bearer {}", token));
	} else if let (Some(username), Some(password)) = (&http.username, &http.password) {
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