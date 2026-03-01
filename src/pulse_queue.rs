use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::utils::PushMessage;

/// Configuration for the pulse retry queue
#[derive(Debug, Clone)]
pub struct PulseQueueConfig {
	pub max_queue_size: usize,
	pub max_retries: u32,
	pub retry_delay_ms: u64,
}

impl Default for PulseQueueConfig {
	fn default() -> Self {
		Self {
			max_queue_size: 10_000,
			max_retries: 300,
			retry_delay_ms: 1000,
		}
	}
}

impl PulseQueueConfig {
	pub fn from_env() -> Self {
		Self {
			max_queue_size: std::env::var("PULSE_MAX_QUEUE_SIZE")
				.ok()
				.and_then(|v| v.parse().ok())
				.unwrap_or(10_000),
			max_retries: std::env::var("PULSE_MAX_RETRIES")
				.ok()
				.and_then(|v| v.parse().ok())
				.unwrap_or(300),
			retry_delay_ms: std::env::var("PULSE_RETRY_DELAY_MS")
				.ok()
				.and_then(|v| v.parse().ok())
				.unwrap_or(1000),
		}
	}

	pub fn cached() -> &'static Self {
		static CONFIG: OnceLock<PulseQueueConfig> = OnceLock::new();
		CONFIG.get_or_init(Self::from_env)
	}
}

#[derive(Debug, Clone)]
struct QueuedPulse {
	message: PushMessage,
	attempts: u32,
	last_sent: Option<Instant>,
}

struct PulseQueueInner {
	/// Insertion-ordered pulse IDs
	order: VecDeque<String>,
	/// Pulse data keyed by pulse_id for O(1) lookup/removal
	pulses: HashMap<String, QueuedPulse>,
}

#[derive(Clone)]
pub struct PulseQueue {
	inner: Arc<Mutex<PulseQueueInner>>,
	config: PulseQueueConfig,
}

impl PulseQueue {
	pub fn new(config: PulseQueueConfig) -> Self {
		let capacity = config.max_queue_size;
		Self {
			inner: Arc::new(Mutex::new(PulseQueueInner {
				order: VecDeque::with_capacity(capacity),
				pulses: HashMap::with_capacity(capacity),
			})),
			config,
		}
	}

	/// Enqueue a pulse for delivery. Assigns a unique pulseId.
	/// Returns the pulseId. If the queue is full, the oldest pulse is dropped.
	pub async fn enqueue(&self, mut message: PushMessage) -> String {
		let mut inner = self.inner.lock().await;

		if inner.pulses.len() >= self.config.max_queue_size {
			while let Some(old_id) = inner.order.pop_front() {
				if let Some(dropped) = inner.pulses.remove(&old_id) {
					warn!(
						"Pulse queue full ({}), dropping oldest pulse {} for token {}",
						self.config.max_queue_size, old_id, dropped.message.token
					);
					break;
				}
			}
		}

		let pulse_id = Uuid::new_v4().to_string();
		message.pulse_id = Some(pulse_id.clone());

		inner.order.push_back(pulse_id.clone());
		inner.pulses.insert(
			pulse_id.clone(),
			QueuedPulse {
				message,
				attempts: 0,
				last_sent: None,
			},
		);

		pulse_id
	}

	/// Acknowledge a successfully delivered pulse by its pulseId O(1).
	/// Returns true if the pulse was found and removed.
	pub async fn acknowledge(&self, pulse_id: &str) -> bool {
		let mut inner = self.inner.lock().await;
		let removed = inner.pulses.remove(pulse_id).is_some();

		if removed {
			debug!("Pulse {} acknowledged and removed from queue", pulse_id);
		}
		removed
	}

	/// Get the next pulse to send, incrementing its attempt counter.
	/// Rotates the queue to prevent starvation.
	pub async fn next_to_send(&self) -> Option<PushMessage> {
		let mut inner = self.inner.lock().await;

		loop {
			let id = inner.order.pop_front()?;

			let Some(pulse) = inner.pulses.get_mut(&id) else {
				// stale id
				continue;
			};

			if pulse.attempts >= self.config.max_retries {
				if let Some(removed) = inner.pulses.remove(&id) {
					warn!(
						"Pulse {} exceeded max retries ({}), dropping for token {}",
						id, self.config.max_retries, removed.message.token
					);
				}
				continue;
			}

			pulse.attempts += 1;

			if pulse.attempts > 1 {
				debug!(
					"Retrying pulse {} (attempt {}/{})",
					id, pulse.attempts, self.config.max_retries
				);
			}

			let msg = pulse.message.clone();

			inner.order.push_back(id);

			return Some(msg);
		}
	}

	pub async fn next_batch_to_send(&self, max: usize) -> Vec<PushMessage> {
		let mut inner = self.inner.lock().await;
		let now = Instant::now();
		let retry_delay = std::time::Duration::from_millis(self.config.retry_delay_ms);

		let mut out = Vec::with_capacity(max);
		let mut scanned = 0;
		let order_len = inner.order.len();

		while scanned < order_len && out.len() < max {
			let Some(id) = inner.order.pop_front() else {
				break;
			};
			scanned += 1;

			let Some(pulse) = inner.pulses.get_mut(&id) else {
				// Stale (already acknowledged) (do not push back)
				continue;
			};

			if pulse.attempts >= self.config.max_retries {
				let token = pulse.message.token.clone();
				inner.pulses.remove(&id);
				warn!(
					"Pulse {} exceeded max retries ({}), dropping for token {}",
					id, self.config.max_retries, token
				);
				continue;
			}

			let ready = match pulse.last_sent {
				None => true,
				Some(t) => now.duration_since(t) >= retry_delay,
			};

			if ready {
				pulse.attempts += 1;
				pulse.last_sent = Some(now);

				if pulse.attempts > 1 {
					debug!(
						"Retrying pulse {} (attempt {}/{})",
						id, pulse.attempts, self.config.max_retries
					);
				}

				out.push(pulse.message.clone());
			}

			// Still pending until ack (push back for next pass)
			inner.order.push_back(id);
		}

		out
	}

	/// Remove all pulses that have exceeded max retries
	pub async fn prune_expired(&self) {
		let mut inner = self.inner.lock().await;
		let before = inner.pulses.len();

		inner
			.pulses
			.retain(|_, p| p.attempts < self.config.max_retries);

		let pruned = before - inner.pulses.len();
		if pruned > 0 {
			warn!("Pruned {} pulses that exceeded max retries", pruned);
		}

		let PulseQueueInner { order, pulses } = &mut *inner;
		order.retain(|id| pulses.contains_key(id));
	}

	/*
	pub async fn pending_count(&self) -> usize {
		self.inner.lock().await.pulses.len()
	}

	pub async fn is_empty(&self) -> bool {
		self.inner.lock().await.pulses.is_empty()
	}
	*/

	pub fn retry_delay(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.config.retry_delay_ms)
	}
}
