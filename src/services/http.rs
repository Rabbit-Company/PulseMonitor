use std::error::Error;
use reqwest::Client;

use crate::utils::Monitor;

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
	};

	if let Some(headers) = &http.headers {
		for header in headers {
			for (key, value) in header {
				request = request.header(key, value);
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