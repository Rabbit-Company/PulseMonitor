use crate::heartbeat::send_heartbeat;
use crate::services::{
	http::is_http_online,
	icmp::is_icmp_online,
	imap::is_imap_online,
	minecraft::{is_minecraft_bedrock_online, is_minecraft_java_online},
	mssql::is_mssql_online,
	mysql::is_mysql_online,
	postgresql::is_postgresql_online,
	redis::is_redis_online,
	smtp::is_smtp_online,
	snmp::is_snmp_online,
	tcp::is_tcp_online,
	udp::is_udp_online,
	ws::is_ws_online,
};
use crate::utils::{CheckResult, Config, Monitor, PushMessage};
use chrono::Utc;

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{RwLock, Semaphore, mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant as TokioInstant, sleep_until};
use tracing::{error, info, warn};

fn round_to_3_decimals(dec: f64) -> f64 {
	(dec * 1000.0).round() / 1000.0
}

/// Type alias for the pulse sender
pub type PulseSender = Arc<RwLock<Option<mpsc::Sender<PushMessage>>>>;

#[derive(Clone)]
struct MonitorEntry {
	monitor: Monitor,
}

#[derive(Clone)]
struct DueItem {
	when: TokioInstant,
	key: String, // token or name
}

impl Ord for DueItem {
	fn cmp(&self, other: &Self) -> Ordering {
		other
			.when
			.cmp(&self.when)
			.then_with(|| other.key.cmp(&self.key))
	}
}
impl PartialOrd for DueItem {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
impl PartialEq for DueItem {
	fn eq(&self, other: &Self) -> bool {
		self.when == other.when && self.key == other.key
	}
}
impl Eq for DueItem {}

struct SchedulerHandle {
	config_tx: watch::Sender<Config>,
	#[allow(unused)]
	stop_tx: oneshot::Sender<()>,
	#[allow(unused)]
	handle: JoinHandle<()>,
}

/// Manages running monitors via a single centralized scheduler task
pub struct MonitorRunner {
	server_url: Option<String>,
	pulse_sender: Option<PulseSender>,
	scheduler: Arc<RwLock<Option<SchedulerHandle>>>,
}

impl MonitorRunner {
	/// Create a new MonitorRunner for file mode (no server_url needed)
	pub fn new() -> Self {
		MonitorRunner {
			server_url: None,
			pulse_sender: None,
			scheduler: Arc::new(RwLock::new(None)),
		}
	}

	/// Create a new MonitorRunner for WebSocket mode with server_url and pulse sender
	pub fn with_websocket(server_url: String, pulse_sender: PulseSender) -> Self {
		MonitorRunner {
			server_url: Some(server_url),
			pulse_sender: Some(pulse_sender),
			scheduler: Arc::new(RwLock::new(None)),
		}
	}

	/// Start monitors from config.
	pub async fn start_monitors(&self, config: &Config) -> Vec<JoinHandle<()>> {
		let max_concurrent_checks = config
			.max_concurrent_checks
			.or_else(|| {
				std::env::var("PULSE_MAX_CONCURRENT_CHECKS")
					.ok()
					.and_then(|s| s.parse().ok())
			})
			.unwrap_or(5000);

		// Start scheduler if needed
		let mut maybe_handle = None;
		{
			let mut guard = self.scheduler.write().await;
			if guard.is_none() {
				let (config_tx, config_rx) = watch::channel(config.clone());
				let (stop_tx, stop_rx) = oneshot::channel::<()>();

				let jitter_ms_max: u64 = 500;

				let server_url = self.server_url.clone();
				let pulse_sender = self.pulse_sender.clone();

				let handle = tokio::spawn(async move {
					run_scheduler_loop(
						config_rx,
						stop_rx,
						server_url,
						pulse_sender,
						max_concurrent_checks,
						jitter_ms_max,
					)
					.await;
				});

				*guard = Some(SchedulerHandle {
					config_tx,
					stop_tx,
					handle,
				});

				maybe_handle = None;
			}
		}

		// Push config into running scheduler (even if it was just created)
		{
			let guard = self.scheduler.read().await;
			if let Some(s) = guard.as_ref() {
				let _ = s.config_tx.send(config.clone());
			}
		}

		maybe_handle.into_iter().collect()
	}

	#[allow(unused)]
	pub async fn stop_all(&self) {
		let sched = { self.scheduler.write().await.take() };
		if let Some(s) = sched {
			info!("Stopping scheduler...");
			let _ = s.stop_tx.send(());
			let _ = s.handle.await;
		}
	}

	/// Update monitors with new config
	pub async fn update_monitors(&self, config: &Config) -> Vec<JoinHandle<()>> {
		info!("Updating monitors with new configuration...");

		self.start_monitors(config).await
	}
}

async fn run_scheduler_loop(
	mut config_rx: watch::Receiver<Config>,
	mut stop_rx: oneshot::Receiver<()>,
	server_url: Option<String>,
	pulse_sender: Option<PulseSender>,
	max_concurrent_checks: usize,
	jitter_ms_max: u64,
) {
	let sem = Arc::new(Semaphore::new(max_concurrent_checks));

	let mut entries: HashMap<String, MonitorEntry> = HashMap::new();
	let mut heap: BinaryHeap<DueItem> = BinaryHeap::new();

	rebuild_state(&config_rx.borrow(), &mut entries, &mut heap, jitter_ms_max);

	loop {
		tokio::select! {
			_ = &mut stop_rx => {
				info!("Scheduler received stop signal");
				break;
			}

			changed = config_rx.changed() => {
				if changed.is_err() {
					warn!("Config channel closed; stopping scheduler");
					break;
				}
				let cfg = config_rx.borrow().clone();
				rebuild_state(&cfg, &mut entries, &mut heap, jitter_ms_max);
				info!("Scheduler applied new config: {} monitors", entries.len());
			}

			_ = async {
				// sleep until next due item, or a short interval if empty
				if let Some(next) = heap.peek().cloned() {
					let now = TokioInstant::now();
					if next.when > now {
						sleep_until(next.when).await;
					} else {
						// already due; yield to run dispatch quickly
						tokio::task::yield_now().await;
					}
				} else {
					// no monitors => sleep a bit
					tokio::time::sleep(Duration::from_millis(200)).await;
				}
			} => {
				dispatch_due(&entries, &mut heap, sem.clone(), server_url.clone(), pulse_sender.clone(), jitter_ms_max).await;
			}
		}
	}
}

fn rebuild_state(
	cfg: &Config,
	entries: &mut HashMap<String, MonitorEntry>,
	heap: &mut BinaryHeap<DueItem>,
	jitter_ms_max: u64,
) {
	entries.clear();
	heap.clear();

	let now = TokioInstant::now();

	for m in &cfg.monitors {
		if !m.enabled {
			continue;
		}
		let key = m.token.clone().unwrap_or_else(|| m.name.clone());

		entries.insert(key.clone(), MonitorEntry { monitor: m.clone() });

		// schedule first run "soon" with jitter to spread load
		let jitter = stable_jitter_ms(&key, jitter_ms_max);
		let first = now + Duration::from_millis(jitter);

		heap.push(DueItem { when: first, key });
	}
}

async fn dispatch_due(
	entries: &HashMap<String, MonitorEntry>,
	heap: &mut BinaryHeap<DueItem>,
	sem: Arc<Semaphore>,
	server_url: Option<String>,
	pulse_sender: Option<PulseSender>,
	jitter_ms_max: u64,
) {
	let now = TokioInstant::now();

	const MAX_DUE_PER_TICK: usize = 20_000;
	let mut processed = 0usize;

	while processed < MAX_DUE_PER_TICK {
		let Some(top) = heap.peek().cloned() else {
			break;
		};
		if top.when > now {
			break;
		}
		let item = heap.pop().unwrap();

		let Some(entry) = entries.get(&item.key) else {
			continue;
		};

		// reschedule next run from "now" to avoid catch-up storms
		let interval = Duration::from_secs(entry.monitor.interval);
		let jitter = stable_jitter_ms(&item.key, jitter_ms_max);
		let next_when = TokioInstant::now() + interval + Duration::from_millis(jitter);
		heap.push(DueItem {
			when: next_when,
			key: item.key.clone(),
		});

		// bounded concurrency: if no permits, requeue soon and move on
		let permit = match sem.clone().try_acquire_owned() {
			Ok(p) => p,
			Err(_) => {
				heap.push(DueItem {
					when: TokioInstant::now() + Duration::from_millis(50),
					key: item.key,
				});
				processed += 1;
				continue;
			}
		};

		let monitor = entry.monitor.clone();
		let server_url = server_url.clone();
		let pulse_sender = pulse_sender.clone();

		tokio::spawn(async move {
			let _permit = permit;
			run_single_check(&monitor, server_url.as_deref(), pulse_sender.as_ref()).await;
		});

		processed += 1;
	}
}

/// Stable jitter based on hashing a string key.
/// This avoids needing RNG and remains stable across restarts.
fn stable_jitter_ms(key: &str, jitter_ms_max: u64) -> u64 {
	if jitter_ms_max == 0 {
		return 0;
	}
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	key.hash(&mut hasher);
	(hasher.finish() % (jitter_ms_max + 1)) as u64
}

async fn run_single_check(
	monitor: &Monitor,
	server_url: Option<&str>,
	pulse_sender: Option<&PulseSender>,
) {
	let start_check_time = Utc::now();
	let start_time = Instant::now();

	let result: Result<CheckResult, Box<dyn std::error::Error + Send + Sync>> =
		if monitor.http.is_some() {
			is_http_online(monitor).await
		} else if monitor.ws.is_some() {
			is_ws_online(monitor).await
		} else if monitor.tcp.is_some() {
			is_tcp_online(monitor).await
		} else if monitor.udp.is_some() {
			is_udp_online(monitor).await
		} else if monitor.icmp.is_some() {
			is_icmp_online(monitor).await
		} else if monitor.smtp.is_some() {
			is_smtp_online(monitor).await
		} else if monitor.imap.is_some() {
			is_imap_online(monitor).await
		} else if monitor.mysql.is_some() {
			is_mysql_online(monitor).await
		} else if monitor.mssql.is_some() {
			is_mssql_online(monitor).await
		} else if monitor.postgresql.is_some() {
			is_postgresql_online(monitor).await
		} else if monitor.redis.is_some() {
			is_redis_online(monitor).await
		} else if monitor.minecraft_java.is_some() {
			is_minecraft_java_online(monitor).await
		} else if monitor.minecraft_bedrock.is_some() {
			is_minecraft_bedrock_online(monitor).await
		} else if monitor.snmp.is_some() {
			is_snmp_online(monitor).await
		} else {
			Ok(CheckResult::from_latency(None))
		};

	let end_check_time = Utc::now();

	match &result {
		Ok(check_result) => {
			let latency_ms = match check_result.latency() {
				Some(latency) => round_to_3_decimals(latency),
				None => round_to_3_decimals(start_time.elapsed().as_secs_f64() * 1000.0),
			};

			if monitor.debug.unwrap_or(false) {
				info!("Monitor '{}' succeed ({}ms)", monitor.name, latency_ms);
			}

			if let Err(e) = send_heartbeat(
				monitor,
				server_url,
				pulse_sender,
				start_check_time,
				end_check_time,
				latency_ms,
				check_result,
			)
			.await
			{
				error!("Failed to send heartbeat for '{}': {}", monitor.name, e);
			}
		}
		Err(err) => {
			if monitor.debug.unwrap_or(false) {
				error!("Monitor '{}' failed: {}", monitor.name, err);
			}
		}
	}
}
