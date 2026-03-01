use crate::pulse_queue::PulseQueueConfig;
use crate::utils::{
	CheckResult, HeartbeatConfig, Monitor, PushMessage, resolve_custom_placeholders,
};
use crate::ws_client::PulseSender;
use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::Client;
use std::error::Error;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{info, warn};

fn build_heartbeat_url(server_url: &str, token: &str) -> String {
	let base_url = server_url.trim_end_matches('/');
	format!(
		"{}/v1/push/{}?latency={{latency}}&startTime={{startTimeISO}}&endTime={{endTimeISO}}&custom1={{custom1}}&custom2={{custom2}}&custom3={{custom3}}",
		base_url, token
	)
}

fn http_client() -> &'static Client {
	static CLIENT: OnceLock<Client> = OnceLock::new();
	CLIENT.get_or_init(|| {
		Client::builder()
			.timeout(Duration::from_secs(10))
			.pool_max_idle_per_host(256)
			.build()
			.expect("Failed to build HTTP client")
	})
}

fn apply_templates(
	template: &str,
	latency_str: &str,
	start_time_iso: &str,
	end_time_iso: &str,
	start_time_unix: &str,
	end_time_unix: &str,
	custom_placeholders: &[(String, String)],
) -> String {
	let mut result = template
		.replace("{latency}", latency_str)
		.replace("{startTimeISO}", start_time_iso)
		.replace("{endTimeISO}", end_time_iso)
		.replace("{startTimeUnix}", start_time_unix)
		.replace("{endTimeUnix}", end_time_unix);

	for (placeholder, value) in custom_placeholders {
		result = result.replace(placeholder, value);
	}

	result
}

/// Send heartbeat using explicit HeartbeatConfig (file mode)
pub async fn send_heartbeat_with_config(
	heartbeat: &HeartbeatConfig,
	start_check_time: DateTime<Utc>,
	end_check_time: DateTime<Utc>,
	latency_ms: f64,
	custom_placeholders: &[(String, String)],
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let client = http_client();

	let latency_str = latency_ms.to_string();
	let start_time_unix = start_check_time.timestamp_millis().to_string();
	let end_time_unix = end_check_time.timestamp_millis().to_string();
	let start_time_iso = start_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);
	let end_time_iso = end_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);

	let url = apply_templates(
		&heartbeat.url,
		&latency_str,
		&start_time_iso,
		&end_time_iso,
		&start_time_unix,
		&end_time_unix,
		custom_placeholders,
	);

	let mut request = match heartbeat.method.to_uppercase().as_str() {
		"GET" => client.get(&url),
		"POST" => client.post(&url),
		"HEAD" => client.head(&url),
		_ => return Err(format!("Unsupported HTTP method: {}", heartbeat.method).into()),
	};

	if let Some(headers) = &heartbeat.headers {
		for header in headers {
			for (key, value) in header {
				let value_with_templates = apply_templates(
					value,
					&latency_str,
					&start_time_iso,
					&end_time_iso,
					&start_time_unix,
					&end_time_unix,
					custom_placeholders,
				);
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

/// Send heartbeat using token and server_url via HTTP (fallback for WebSocket mode)
pub async fn send_heartbeat_with_token_http(
	server_url: &str,
	token: &str,
	start_check_time: DateTime<Utc>,
	end_check_time: DateTime<Utc>,
	latency_ms: f64,
	custom_placeholders: &[(String, String)],
	max_retries: u32,
	retry_delay_ms: u64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let client = http_client();

	let latency_str = latency_ms.to_string();
	let start_time_unix = start_check_time.timestamp_millis().to_string();
	let end_time_unix = end_check_time.timestamp_millis().to_string();
	let start_time_iso = start_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);
	let end_time_iso = end_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);

	let url_template = build_heartbeat_url(server_url, token);
	let url = apply_templates(
		&url_template,
		&latency_str,
		&start_time_iso,
		&end_time_iso,
		&start_time_unix,
		&end_time_unix,
		custom_placeholders,
	);

	let mut last_error = None;
	for attempt in 1..=max_retries + 1 {
		match client.get(&url).send().await {
			Ok(response) if response.status().is_success() => {
				if attempt > 1 {
					info!("HTTP pulse succeeded on attempt {}", attempt);
				}
				return Ok(());
			}
			Ok(response) => {
				last_error = Some(format!("HTTP {} on attempt {}", response.status(), attempt));
				warn!("{}", last_error.as_ref().unwrap());
			}
			Err(e) => {
				last_error = Some(format!("Request error on attempt {}: {}", attempt, e));
				warn!("{}", last_error.as_ref().unwrap());
			}
		}

		if attempt <= max_retries {
			sleep(Duration::from_millis(retry_delay_ms)).await;
		}
	}

	Err(
		last_error
			.unwrap_or_else(|| "Unknown error".to_string())
			.into(),
	)
}

/// Send heartbeat using WebSocket connection
pub async fn send_heartbeat_via_websocket(
	pulse_sender: &Arc<RwLock<Option<PulseSender>>>,
	token: &str,
	start_check_time: DateTime<Utc>,
	end_check_time: DateTime<Utc>,
	latency_ms: f64,
	check_result: &CheckResult,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let start_time_iso = start_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);
	let end_time_iso = end_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);

	let push_message = PushMessage::new(
		token,
		Some(latency_ms),
		Some(start_time_iso),
		Some(end_time_iso),
	)
	.with_custom_metrics(check_result);

	// Get the sender and send the message
	let sender_guard = pulse_sender.read().await;
	if let Some(sender) = sender_guard.as_ref() {
		match sender.try_send(push_message) {
			Ok(_) => Ok(()),
			Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
				// At scale, prefer dropping over blocking checks.
				warn!(
					"WebSocket pulse channel full; dropping pulse for token {}",
					token
				);
				Ok(())
			}
			Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
				Err("WebSocket pulse channel closed".into())
			}
		}
	} else {
		Err("WebSocket pulse channel not available".into())
	}
}

/// Send heartbeat for a monitor - handles file mode, WebSocket mode, and HTTP fallback
pub async fn send_heartbeat(
	monitor: &Monitor,
	server_url: Option<&str>,
	pulse_sender: Option<&Arc<RwLock<Option<PulseSender>>>>,
	start_check_time: DateTime<Utc>,
	end_check_time: DateTime<Utc>,
	latency_ms: f64,
	check_result: &CheckResult,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let custom_placeholders = resolve_custom_placeholders(monitor, check_result);

	// File mode: use explicit heartbeat config
	if let Some(ref heartbeat) = monitor.heartbeat {
		return send_heartbeat_with_config(
			heartbeat,
			start_check_time,
			end_check_time,
			latency_ms,
			&custom_placeholders,
		)
		.await;
	}

	// WebSocket mode: try to send via WebSocket first
	if let (Some(token), Some(pulse_tx)) = (&monitor.token, pulse_sender) {
		match send_heartbeat_via_websocket(
			pulse_tx,
			token,
			start_check_time,
			end_check_time,
			latency_ms,
			check_result,
		)
		.await
		{
			Ok(_) => return Ok(()),
			Err(e) => {
				// WebSocket send failed, fall back to HTTP if server_url is available
				warn!("WebSocket pulse failed ({}), falling back to HTTP", e);
				if let Some(url) = server_url {
					let cfg = PulseQueueConfig::cached();
					return send_heartbeat_with_token_http(
						url,
						token,
						start_check_time,
						end_check_time,
						latency_ms,
						&custom_placeholders,
						cfg.max_retries,
						cfg.retry_delay_ms,
					)
					.await;
				}
				return Err(e);
			}
		}
	}

	// HTTP-only mode (WebSocket mode without active connection)
	if let (Some(token), Some(url)) = (&monitor.token, server_url) {
		let cfg = PulseQueueConfig::cached();
		return send_heartbeat_with_token_http(
			url,
			token,
			start_check_time,
			end_check_time,
			latency_ms,
			&custom_placeholders,
			cfg.max_retries,
			cfg.retry_delay_ms,
		)
		.await;
	}

	Err("No heartbeat configuration: need either heartbeat config or token + server_url".into())
}
