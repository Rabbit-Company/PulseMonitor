use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

use crate::utils::{Config, WsMessage};

const RECONNECT_DELAY_SECS: u64 = 3;

/// Convert HTTP(S) URL to WebSocket URL
fn http_to_ws_url(url: &str) -> String {
	let ws_url = url
		.trim_end_matches('/')
		.replace("https://", "wss://")
		.replace("http://", "ws://");

	format!("{}/ws", ws_url)
}

/// WebSocket client that maintains connection to UptimeMonitor-Server
pub struct WsClient {
	ws_url: String,
	token: String,
}

impl WsClient {
	pub fn new(server_url: &str, token: &str) -> Self {
		let ws_url = http_to_ws_url(server_url);
		info!("WebSocket URL: {}", ws_url);

		WsClient {
			ws_url,
			token: token.to_string(),
		}
	}

	/// Start the WebSocket client and return a receiver for config updates
	pub async fn start(self: Arc<Self>) -> mpsc::Receiver<Config> {
		let (tx, rx) = mpsc::channel::<Config>(32);

		tokio::spawn(async move {
			self.connection_loop(tx).await;
		});

		rx
	}

	async fn connection_loop(&self, config_tx: mpsc::Sender<Config>) {
		loop {
			match self.connect_and_subscribe(&config_tx).await {
				Ok(_) => {
					warn!(
						"WebSocket connection closed, reconnecting in {} seconds...",
						RECONNECT_DELAY_SECS
					);
				}
				Err(e) => {
					error!(
						"WebSocket error: {}, reconnecting in {} seconds...",
						e, RECONNECT_DELAY_SECS
					);
				}
			}

			sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
		}
	}

	async fn connect_and_subscribe(
		&self,
		config_tx: &mpsc::Sender<Config>,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		info!("Connecting to WebSocket server: {}", self.ws_url);

		let (ws_stream, _) = connect_async(&self.ws_url).await?;
		let (mut write, mut read) = ws_stream.split();

		info!("Connected to WebSocket server, subscribing...");

		// Send subscribe message
		let subscribe_msg = WsMessage::subscribe(&self.token);
		let subscribe_json = serde_json::to_string(&subscribe_msg)?;
		write.send(Message::Text(subscribe_json.into())).await?;

		// Listen for messages
		while let Some(msg_result) = read.next().await {
			match msg_result {
				Ok(Message::Text(text)) => {
					if let Err(e) = self.handle_message(&text, config_tx).await {
						error!("Failed to handle message: {}", e);
					}
				}
				Ok(Message::Ping(data)) => {
					if let Err(e) = write.send(Message::Pong(data)).await {
						error!("Failed to send pong: {}", e);
						break;
					}
				}
				Ok(Message::Close(_)) => {
					info!("Server closed the connection");
					break;
				}
				Err(e) => {
					error!("WebSocket error: {}", e);
					break;
				}
				_ => {}
			}
		}

		Ok(())
	}

	async fn handle_message(
		&self,
		text: &str,
		config_tx: &mpsc::Sender<Config>,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		let message: WsMessage = serde_json::from_str(text)?;

		match message {
			WsMessage::Connected {
				message: _,
				timestamp: _,
			} => {}
			WsMessage::Subscribed {
				pulse_monitor_id,
				pulse_monitor_name,
				data,
				timestamp: _,
			} => {
				info!(
					"Subscription successful: {} ({})",
					pulse_monitor_name, pulse_monitor_id
				);
				info!("Received {} monitors from server", data.monitors.len());

				let config = Config {
					monitors: data.monitors,
				};

				if let Err(e) = config_tx.send(config).await {
					error!("Failed to send config update: {}", e);
				}
			}
			WsMessage::Error {
				message,
				timestamp: _,
			} => {
				error!("Server error: {}", message);
				// For invalid token, we might want to exit
				if message.contains("Invalid") {
					return Err(format!("Authentication failed: {}", message).into());
				}
			}
			WsMessage::ConfigUpdate { data, timestamp: _ } => {
				info!(
					"Received configuration update with {} monitors",
					data.monitors.len()
				);

				let config = Config {
					monitors: data.monitors,
				};

				if let Err(e) = config_tx.send(config).await {
					error!("Failed to send config update: {}", e);
				}
			}
			WsMessage::Subscribe { .. } => {
				// This shouldn't be received from server
				warn!("Received unexpected Subscribe message from server");
			}
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_http_to_ws_url() {
		assert_eq!(
			http_to_ws_url("https://pulse.rabbitmonitor.com"),
			"wss://pulse.rabbitmonitor.com/ws"
		);
		assert_eq!(
			http_to_ws_url("https://pulse.rabbitmonitor.com/"),
			"wss://pulse.rabbitmonitor.com/ws"
		);
		assert_eq!(
			http_to_ws_url("http://localhost:3000"),
			"ws://localhost:3000/ws"
		);
	}
}
