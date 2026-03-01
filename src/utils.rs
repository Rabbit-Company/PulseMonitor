use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const VERSION: &str = "v3.15.0";

#[derive(Default, Debug, Clone)]
pub struct CheckResult {
	pub values: HashMap<String, f64>,
}

impl CheckResult {
	pub fn new() -> Self {
		CheckResult {
			values: HashMap::new(),
		}
	}

	pub fn from_latency(latency: Option<f64>) -> Self {
		let mut values = HashMap::new();
		if let Some(l) = latency {
			values.insert("latency".to_string(), l);
		}
		CheckResult { values }
	}

	pub fn latency(&self) -> Option<f64> {
		self.values.get("latency").copied()
	}

	pub fn set(&mut self, key: impl Into<String>, value: f64) {
		self.values.insert(key.into(), value);
	}

	pub fn get(&self, key: &str) -> Option<f64> {
		self.values.get(key).copied()
	}
}

pub fn resolve_custom_placeholders(
	monitor: &Monitor,
	result: &CheckResult,
) -> Vec<(String, String)> {
	let mut placeholders = Vec::new();

	// Always emit {custom1}, {custom2}, {custom3} (empty string if absent)
	for key in &["custom1", "custom2", "custom3"] {
		let value_str = result.get(key).map(|v| v.to_string()).unwrap_or_default();
		placeholders.push((format!("{{{}}}", key), value_str));
	}

	// Minecraft alias
	if monitor.minecraft_java.is_some() || monitor.minecraft_bedrock.is_some() {
		let player_count = result
			.get("playerCount")
			.or_else(|| result.get("custom1"))
			.map(|v| v.to_string())
			.unwrap_or_default();
		placeholders.push(("{playerCount}".to_string(), player_count));
	}

	// Emit all other arbitrary keys from the result
	for (key, value) in &result.values {
		match key.as_str() {
			"latency" | "custom1" | "custom2" | "custom3" | "playerCount" => continue,
			_ => {
				placeholders.push((format!("{{{}}}", key), value.to_string()));
			}
		}
	}

	placeholders
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
	pub monitors: Vec<Monitor>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
	pub enabled: bool,
	pub interval: u64,
	pub name: String,
	/// Token for this monitor (used in WebSocket mode to build heartbeat URL)
	pub token: Option<String>,
	/// Heartbeat configuration (used in file mode, optional in WebSocket mode)
	pub heartbeat: Option<HeartbeatConfig>,
	pub debug: Option<bool>,
	pub http: Option<HttpConfig>,
	pub ws: Option<WsConfig>,
	pub tcp: Option<TcpConfig>,
	pub udp: Option<UdpConfig>,
	pub icmp: Option<IcmpConfig>,
	pub smtp: Option<SmtpConfig>,
	pub imap: Option<ImapConfig>,
	pub mysql: Option<MysqlConfig>,
	pub mssql: Option<MssqlConfig>,
	pub postgresql: Option<PostgreSqlConfig>,
	pub redis: Option<RedisConfig>,
	#[serde(rename = "minecraft-java")]
	pub minecraft_java: Option<MinecraftJavaConfig>,
	#[serde(rename = "minecraft-bedrock")]
	pub minecraft_bedrock: Option<MinecraftBedrockConfig>,
	pub snmp: Option<SnmpConfig>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatConfig {
	pub method: String,
	pub url: String,
	pub timeout: Option<u64>,
	pub headers: Option<Vec<HashMap<String, String>>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlConfig {
	pub url: String,
	pub timeout: Option<u64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlConfig {
	pub url: String,
	pub timeout: Option<u64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgreSqlConfig {
	pub url: String,
	pub timeout: Option<u64>,
	pub use_tls: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisConfig {
	pub url: String,
	pub timeout: Option<u64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpConfig {
	pub method: String,
	pub url: String,
	pub timeout: Option<u64>,
	pub headers: Option<Vec<HashMap<String, String>>>,
	pub json_paths: Option<HashMap<String, String>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsConfig {
	pub url: String,
	pub timeout: Option<u64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpConfig {
	pub host: String,
	pub port: u16,
	pub timeout: Option<u64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UdpConfig {
	pub host: String,
	pub port: u16,
	pub timeout: Option<u64>,
	pub payload: Option<String>,
	pub expect_response: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IcmpConfig {
	pub host: String,
	pub timeout: Option<u64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImapConfig {
	pub server: String,
	pub port: u16,
	pub username: String,
	pub password: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmtpConfig {
	pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftJavaConfig {
	pub host: String,
	pub port: Option<u16>,
	pub timeout: Option<u64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftBedrockConfig {
	pub host: String,
	pub port: Option<u16>,
	pub timeout: Option<u64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnmpConfig {
	pub host: String,
	pub port: Option<u16>,
	pub timeout: Option<u64>,
	/// SNMP version: "1", "2c", or "3" (default: "3")
	pub version: Option<String>,
	/// Community string for v1/v2c (default: "public")
	pub community: Option<String>,
	/// SNMPv3 USM username
	pub username: Option<String>,
	/// SNMPv3 authentication password
	pub auth_password: Option<String>,
	/// SNMPv3 auth protocol: md5, sha1, sha256, etc. (default: "sha256")
	pub auth_protocol: Option<String>,
	/// SNMPv3 privacy password (required for authPriv)
	pub priv_password: Option<String>,
	/// SNMPv3 privacy cipher: des, aes128, aes192, aes256 (default: "aes128")
	pub priv_cipher: Option<String>,
	/// SNMPv3 security level: noAuthNoPriv, authNoPriv, authPriv (default: "authPriv")
	pub security_level: Option<String>,
	/// Primary OID for availability check (default: sysUpTime 1.3.6.1.2.1.1.3.0)
	pub oid: Option<String>,
	/// Map of placeholder name -> OID for querying custom values
	pub oids: Option<HashMap<String, String>>,
}

// WebSocket Protocol Messages

/// Data wrapper for monitor list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorData {
	pub monitors: Vec<Monitor>,
}

/// Push message to send a pulse via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushMessage {
	pub action: String,
	pub token: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pulse_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub latency: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub start_time: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub end_time: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub custom1: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub custom2: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub custom3: Option<f64>,
}

impl PushMessage {
	pub fn new(
		token: &str,
		latency: Option<f64>,
		start_time: Option<String>,
		end_time: Option<String>,
	) -> Self {
		PushMessage {
			action: "push".to_string(),
			token: token.to_string(),
			pulse_id: None,
			latency,
			start_time,
			end_time,
			custom1: None,
			custom2: None,
			custom3: None,
		}
	}

	pub fn with_custom_metrics(mut self, result: &CheckResult) -> Self {
		self.custom1 = result.get("custom1");
		self.custom2 = result.get("custom2");
		self.custom3 = result.get("custom3");
		self
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum WsMessage {
	Connected {
		message: String,
		timestamp: String,
	},
	Subscribe {
		token: String,
	},
	Subscribed {
		#[serde(rename = "pulseMonitorId")]
		pulse_monitor_id: String,
		#[serde(rename = "pulseMonitorName")]
		pulse_monitor_name: String,
		data: MonitorData,
		timestamp: String,
	},
	Error {
		message: String,
		timestamp: String,
	},
	#[serde(rename = "config-update")]
	ConfigUpdate {
		data: MonitorData,
		timestamp: String,
	},
	Pushed {
		#[serde(rename = "pulseId")]
		pulse_id: Option<String>,
		#[serde(rename = "monitorId")]
		monitor_id: String,
		timestamp: String,
	},
}

impl WsMessage {
	pub fn subscribe(token: &str) -> Self {
		WsMessage::Subscribe {
			token: token.to_string(),
		}
	}
}
