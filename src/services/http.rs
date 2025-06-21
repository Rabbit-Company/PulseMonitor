use reqwest::Client;
use std::{
	error::Error,
	time::{Duration, Instant},
};

use crate::utils::Monitor;

pub async fn is_http_online(
	monitor: &Monitor,
) -> Result<Option<f64>, Box<dyn Error + Send + Sync>> {
	let http = monitor
		.http
		.as_ref()
		.ok_or("Monitor does not contain HTTP configuration")?;

	let client = Client::builder()
		.timeout(Duration::from_secs(http.timeout.unwrap_or(10)))
		.build()?;

	let mut request = match http.method.to_uppercase().as_str() {
		"GET" => client.get(&http.url),
		"POST" => client.post(&http.url),
		"HEAD" => client.head(&http.url),
		_ => return Err(format!("Unsupported HTTP method: {}", http.method).into()),
	};

	if let Some(headers) = &http.headers {
		for header in headers {
			for (key, value) in header {
				request = request.header(key, value);
			}
		}
	}

	let request_start = Instant::now();
	let response = request.send().await?;
	let request_latency = request_start.elapsed().as_secs_f64() * 1000.0;

	if response.status().is_success() {
		Ok(Some(request_latency))
	} else {
		Err(format!("Request failed with status: {}", response.status()).into())
	}
}
