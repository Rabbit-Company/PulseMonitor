use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const VERSION: &str = "v3.14.0";

#[derive(Default, Debug, Clone)]
pub struct CheckResult {
	pub latency: Option<f64>,
	pub custom1: Option<f64>,
	pub custom2: Option<f64>,
	pub custom3: Option<f64>,
}

impl CheckResult {
	pub fn from_latency(latency: Option<f64>) -> Self {
		CheckResult {
			latency,
			custom1: None,
			custom2: None,
			custom3: None,
		}
	}
}

pub fn resolve_custom_placeholders(
	monitor: &Monitor,
	result: &CheckResult,
) -> Vec<(&'static str, String)> {
	let mut placeholders = Vec::new();

	let custom1_str = result.custom1.map(|v| v.to_string()).unwrap_or_default();
	let custom2_str = result.custom2.map(|v| v.to_string()).unwrap_or_default();
	let custom3_str = result.custom3.map(|v| v.to_string()).unwrap_or_default();

	placeholders.push(("{custom1}", custom1_str.clone()));
	placeholders.push(("{custom2}", custom2_str.clone()));
	placeholders.push(("{custom3}", custom3_str.clone()));

	if monitor.minecraft_java.is_some() || monitor.minecraft_bedrock.is_some() {
		// Minecraft: custom1 = player count
		placeholders.push(("{playerCount}", custom1_str));
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
	/// OID to query {custom1}
	pub custom1_oid: Option<String>,
	/// OID to query {custom2}
	pub custom2_oid: Option<String>,
	/// OID to query {custom3}
	pub custom3_oid: Option<String>,
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
			latency,
			start_time,
			end_time,
			custom1: None,
			custom2: None,
			custom3: None,
		}
	}

	#[allow(dead_code)]
	pub fn with_custom_metrics(
		mut self,
		custom1: Option<f64>,
		custom2: Option<f64>,
		custom3: Option<f64>,
	) -> Self {
		self.custom1 = custom1;
		self.custom2 = custom2;
		self.custom3 = custom3;
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
