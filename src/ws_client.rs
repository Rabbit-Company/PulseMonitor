use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

use crate::pulse_queue::{PulseQueue, PulseQueueConfig};
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
	pulse_queue: PulseQueue,
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
			pulse_queue: PulseQueue::new(PulseQueueConfig::from_env()),
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
		let (pulse_tx, mut pulse_rx) = mpsc::channel::<PushMessage>(4096);

		// Store the pulse sender for monitors to use
		{
			let mut tx = self.pulse_tx.write().await;
			*tx = Some(pulse_tx);
		}

		info!("WebSocket pulse channel established");

		let retry_delay = self.pulse_queue.retry_delay();

		// Listen for messages and handle pulse sends
		loop {
			tokio::select! {
				// Handle incoming WebSocket messages
				msg_result = read.next() => {
					match msg_result {
						Some(Ok(Message::Text(text))) => {
							match serde_json::from_str::<WsMessage>(&text) {
								Ok(ws_msg) => {
									if let Err(e) = self.handle_parsed_message(ws_msg, config_tx).await {
										error!("Failed to handle WS message: {}", e);
									}
								}
								Err(e) => error!("Failed to parse WebSocket message: {}", e),
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
							// Enqueue the pulse (assigns pulseId)
							self.pulse_queue.enqueue(push_message).await;

							// Send the next pulse from the queue
							if let Some(msg) = self.pulse_queue.next_to_send().await {
								match serde_json::to_string(&msg) {
									Ok(json) => {
										if let Err(e) = write.send(Message::Text(json.into())).await {
											error!("Failed to send pulse via WebSocket: {}", e);
											break;
										}
									}
									Err(e) => error!("Failed to serialize pulse: {}", e),
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

				// Retry timer -> periodically resend unacknowledged pulses
				_ = sleep(retry_delay) => {
					self.pulse_queue.prune_expired().await;

					// cap per tick to avoid huge bursts
					const MAX_RETRIES_PER_TICK: usize = 2000;

					let batch = self.pulse_queue.next_batch_to_send(MAX_RETRIES_PER_TICK).await;
					for msg in batch {
						match serde_json::to_string(&msg) {
							Ok(json) => {
								if let Err(e) = write.send(Message::Text(json.into())).await {
									error!("Failed to retry pulse: {}", e);
									break;
								}
							}
							Err(e) => error!("Failed to serialize retry pulse: {}", e),
						}
					}
				}

			}
		}

		Ok(())
	}

	async fn handle_parsed_message(
		&self,
		message: WsMessage,
		config_tx: &mpsc::Sender<Config>,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		match message {
			WsMessage::Connected { .. } => {}

			WsMessage::Subscribed {
				pulse_monitor_id,
				pulse_monitor_name,
				data,
				..
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

			WsMessage::Error { message, .. } => {
				error!("Server error: {}", message);

				// For invalid token, we might want to exit
				if message.contains("Invalid") {
					return Err(format!("Authentication failed: {}", message).into());
				}
			}

			WsMessage::ConfigUpdate { data, .. } => {
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

			WsMessage::Pushed { pulse_id, .. } => {
				if let Some(pid) = pulse_id {
					self.pulse_queue.acknowledge(&pid).await;
				}
			}

			WsMessage::Subscribe { .. } => {
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
