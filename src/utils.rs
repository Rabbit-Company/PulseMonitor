use std::collections::HashMap;

use serde::{Serialize,Deserialize};

pub const VERSION: &str = "v3.5.0";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
  pub monitors: Vec<Monitor>
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
	pub enabled: bool,
	pub interval: u64,
	pub name: String,
	pub heartbeat: HeartbeatConfig,
	pub debug: Option<bool>,
	pub http: Option<HttpConfig>,
	pub tcp: Option<TcpConfig>,
	pub udp: Option<UdpConfig>,
	pub icmp: Option<IcmpConfig>,
	pub smtp: Option<SmtpConfig>,
	pub imap: Option<ImapConfig>,
	pub mysql: Option<MysqlConfig>,
	pub mssql: Option<MssqlConfig>,
	pub postgresql: Option<PostgreSqlConfig>,
	pub redis: Option<RedisConfig>
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatConfig {
	pub method: String,
	pub url: String,
	pub headers: Option<Vec<HashMap<String, String>>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlConfig {
	pub url: String,
	pub timeout: Option<u64>
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlConfig {
	pub url: String,
	pub timeout: Option<u64>
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgreSqlConfig {
	pub url: String,
	pub timeout: Option<u64>,
	pub use_tls: Option<bool>
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
	pub headers: Option<Vec<HashMap<String, String>>>,
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