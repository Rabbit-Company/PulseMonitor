use crate::utils::Monitor;
use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::Client;
use std::error::Error;
use std::time::Duration;

pub async fn send_heartbeat(
	monitor: &Monitor,
	start_check_time: DateTime<Utc>,
	end_check_time: DateTime<Utc>,
	latency_ms: f64,
) -> Result<(), Box<dyn Error>> {
	let client = Client::builder()
		.timeout(Duration::from_secs(monitor.heartbeat.timeout.unwrap_or(10)))
		.build()?;

	let latency_str = latency_ms.to_string();
	let start_time_unix = start_check_time.timestamp_millis().to_string();
	let end_time_unix = end_check_time.timestamp_millis().to_string();
	let start_time_iso = start_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);
	let end_time_iso = end_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);

	let url = monitor
		.heartbeat
		.url
		.replace("{latency}", &latency_str)
		.replace("{startTimeISO}", &start_time_iso)
		.replace("{endTimeISO}", &end_time_iso)
		.replace("{startTimeUnix}", &start_time_unix)
		.replace("{endTimeUnix}", &end_time_unix);

	let mut request = match monitor.heartbeat.method.to_uppercase().as_str() {
		"GET" => client.get(&url),
		"POST" => client.post(&url),
		"HEAD" => client.head(&url),
		_ => return Err(format!("Unsupported HTTP method: {}", monitor.heartbeat.method).into()),
	};

	if let Some(headers) = &monitor.heartbeat.headers {
		for header in headers {
			for (key, value) in header {
				let value_with_templates = value
					.replace("{latency}", &latency_str)
					.replace("{startTimeISO}", &start_time_iso)
					.replace("{endTimeISO}", &end_time_iso)
					.replace("{startTimeUnix}", &start_time_unix)
					.replace("{endTimeUnix}", &end_time_unix);
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
