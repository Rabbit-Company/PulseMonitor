use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const VERSION: &str = "v3.12.1";

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
