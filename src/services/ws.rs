use futures_util::{SinkExt, StreamExt};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{error::Error, time::Duration};
use tokio::time::timeout;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::utils::{CheckResult, Monitor};

pub async fn is_ws_online(monitor: &Monitor) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let ws = monitor
		.ws
		.as_ref()
		.ok_or("Monitor does not contain WS configuration")?;

	let timeout_duration = Duration::from_secs(ws.timeout.unwrap_or(3));

	let (stream, _) = connect_async(&ws.url).await?;

	let (mut write, mut read) = stream.split();

	let ping_payload = format!("{:?}", SystemTime::now().duration_since(UNIX_EPOCH)?).into_bytes();

	write
		.send(Message::Ping(ping_payload.clone().into()))
		.await?;

	let response = timeout(timeout_duration, read.next()).await;

	match response {
		Ok(Some(Ok(Message::Pong(payload)))) if payload == ping_payload => {
			write.close().await?;
			Ok(CheckResult::from_latency(None))
		}
		Ok(Some(Ok(msg))) => Err(format!("Unexpected response: {:?}", msg).into()),
		Ok(Some(Err(e))) => Err(format!("Error reading pong: {}", e).into()),
		Ok(None) => Err("Connection closed without pong".into()),
		Err(_) => Err("Timed out waiting for pong".into()),
	}
}
