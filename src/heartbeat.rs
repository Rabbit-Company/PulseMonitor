use crate::utils::{HeartbeatConfig, Monitor, PushMessage};
use crate::ws_client::PulseSender;
use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::Client;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::warn;

fn build_heartbeat_url(server_url: &str, token: &str) -> String {
	let base_url = server_url.trim_end_matches('/');
	format!(
		"{}/v1/push/{}?latency={{latency}}&startTime={{startTimeISO}}&endTime={{endTimeISO}}",
		base_url, token
	)
}

/// Send heartbeat using explicit HeartbeatConfig (file mode)
pub async fn send_heartbeat_with_config(
	heartbeat: &HeartbeatConfig,
	start_check_time: DateTime<Utc>,
	end_check_time: DateTime<Utc>,
	latency_ms: f64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let client = Client::builder()
		.timeout(Duration::from_secs(heartbeat.timeout.unwrap_or(10)))
		.build()?;

	let latency_str = latency_ms.to_string();
	let start_time_unix = start_check_time.timestamp_millis().to_string();
	let end_time_unix = end_check_time.timestamp_millis().to_string();
	let start_time_iso = start_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);
	let end_time_iso = end_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);

	let url = heartbeat
		.url
		.replace("{latency}", &latency_str)
		.replace("{startTimeISO}", &start_time_iso)
		.replace("{endTimeISO}", &end_time_iso)
		.replace("{startTimeUnix}", &start_time_unix)
		.replace("{endTimeUnix}", &end_time_unix);

	let mut request = match heartbeat.method.to_uppercase().as_str() {
		"GET" => client.get(&url),
		"POST" => client.post(&url),
		"HEAD" => client.head(&url),
		_ => return Err(format!("Unsupported HTTP method: {}", heartbeat.method).into()),
	};

	if let Some(headers) = &heartbeat.headers {
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

/// Send heartbeat using token and server_url via HTTP (fallback for WebSocket mode)
pub async fn send_heartbeat_with_token_http(
	server_url: &str,
	token: &str,
	start_check_time: DateTime<Utc>,
	end_check_time: DateTime<Utc>,
	latency_ms: f64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

	let latency_str = latency_ms.to_string();
	let start_time_iso = start_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);
	let end_time_iso = end_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);

	let url_template = build_heartbeat_url(server_url, token);
	let url = url_template
		.replace("{latency}", &latency_str)
		.replace("{startTimeISO}", &start_time_iso)
		.replace("{endTimeISO}", &end_time_iso);

	let response = client.get(&url).send().await?;

	if response.status().is_success() {
		Ok(())
	} else {
		Err(format!("Request failed with status: {}", response.status()).into())
	}
}

/// Send heartbeat using WebSocket connection
pub async fn send_heartbeat_via_websocket(
	pulse_sender: &Arc<RwLock<Option<PulseSender>>>,
	token: &str,
	start_check_time: DateTime<Utc>,
	end_check_time: DateTime<Utc>,
	latency_ms: f64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let start_time_iso = start_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);
	let end_time_iso = end_check_time.to_rfc3339_opts(SecondsFormat::Millis, true);

	let push_message = PushMessage::new(
		token,
		Some(latency_ms),
		Some(start_time_iso),
		Some(end_time_iso),
	);

	// Get the sender and send the message
	let sender_guard = pulse_sender.read().await;
	if let Some(sender) = sender_guard.as_ref() {
		sender
			.send(push_message)
			.await
			.map_err(|e| -> Box<dyn Error + Send + Sync> {
				format!("Failed to send pulse via WebSocket channel: {}", e).into()
			})?;
		Ok(())
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
) -> Result<(), Box<dyn Error + Send + Sync>> {
	// File mode: use explicit heartbeat config
	if let Some(ref heartbeat) = monitor.heartbeat {
		return send_heartbeat_with_config(heartbeat, start_check_time, end_check_time, latency_ms)
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
		)
		.await
		{
			Ok(_) => return Ok(()),
			Err(e) => {
				// WebSocket send failed, fall back to HTTP if server_url is available
				warn!("WebSocket pulse failed ({}), falling back to HTTP", e);
				if let Some(url) = server_url {
					return send_heartbeat_with_token_http(
						url,
						token,
						start_check_time,
						end_check_time,
						latency_ms,
					)
					.await;
				}
				return Err(e);
			}
		}
	}

	// HTTP-only mode (WebSocket mode without active connection)
	if let (Some(token), Some(url)) = (&monitor.token, server_url) {
		return send_heartbeat_with_token_http(
			url,
			token,
			start_check_time,
			end_check_time,
			latency_ms,
		)
		.await;
	}

	Err("No heartbeat configuration: need either heartbeat config or token + server_url".into())
}
