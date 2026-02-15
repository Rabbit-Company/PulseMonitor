use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

use crate::utils::{Config, PushMessage, WsMessage};

const RECONNECT_DELAY_SECS: u64 = 1;

/// Convert HTTP(S) URL to WebSocket URL
fn http_to_ws_url(url: &str) -> String {
	let ws_url = url
		.trim_end_matches('/')
		.replace("https://", "wss://")
		.replace("http://", "ws://");

	format!("{}/ws", ws_url)
}

/// Channel for sending pulses through WebSocket
pub type PulseSender = mpsc::Sender<PushMessage>;

/// WebSocket client that maintains connection to UptimeMonitor-Server
pub struct WsClient {
	ws_url: String,
	token: String,
	/// Shared sender for pulse messages
	pulse_tx: Arc<RwLock<Option<mpsc::Sender<PushMessage>>>>,
}

impl WsClient {
	pub fn new(server_url: &str, token: &str) -> Self {
		let ws_url = http_to_ws_url(server_url);
		info!("WebSocket URL: {}", ws_url);

		WsClient {
			ws_url,
			token: token.to_string(),
			pulse_tx: Arc::new(RwLock::new(None)),
		}
	}

	/// Get a clone of the pulse sender for use by monitors
	pub fn get_pulse_sender(&self) -> Arc<RwLock<Option<mpsc::Sender<PushMessage>>>> {
		Arc::clone(&self.pulse_tx)
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

			// Clear the pulse sender on disconnect
			{
				let mut tx = self.pulse_tx.write().await;
				*tx = None;
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

		// Create channel for pulse messages
		let (pulse_tx, mut pulse_rx) = mpsc::channel::<PushMessage>(256);

		// Store the pulse sender for monitors to use
		{
			let mut tx = self.pulse_tx.write().await;
			*tx = Some(pulse_tx);
		}

		info!("WebSocket pulse channel established");

		// Listen for messages and handle pulse sends
		loop {
			tokio::select! {
				// Handle incoming WebSocket messages
				msg_result = read.next() => {
					match msg_result {
						Some(Ok(Message::Text(text))) => {
							if let Err(e) = self.handle_message(&text, config_tx).await {
								error!("Failed to handle message: {}", e);
							}
						}
						Some(Ok(Message::Ping(data))) => {
							if let Err(e) = write.send(Message::Pong(data)).await {
								error!("Failed to send pong: {}", e);
								break;
							}
						}
						Some(Ok(Message::Close(_))) => {
							info!("Server closed the connection");
							break;
						}
						Some(Err(e)) => {
							error!("WebSocket error: {}", e);
							break;
						}
						None => {
							info!("WebSocket stream ended");
							break;
						}
						_ => {}
					}
				}
				// Handle outgoing pulse messages
				pulse_msg = pulse_rx.recv() => {
					match pulse_msg {
						Some(push_message) => {
							match serde_json::to_string(&push_message) {
								Ok(json) => {
									if let Err(e) = write.send(Message::Text(json.into())).await {
										error!("Failed to send pulse via WebSocket: {}", e);
										break;
									}
								}
								Err(e) => {
									error!("Failed to serialize pulse message: {}", e);
								}
							}
						}
						None => {
							// Channel closed, exit the loop
							warn!("Pulse channel closed");
							break;
						}
					}
				}
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
			WsMessage::Pushed {
				monitor_id: _,
				timestamp: _,
			} => {
				// Pulse acknowledged by server - no action needed
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
