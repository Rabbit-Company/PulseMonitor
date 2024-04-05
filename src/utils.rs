use serde::{Serialize,Deserialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
  pub monitors: Vec<Monitor>
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
	pub enabled: bool,
	#[serde(rename = "execute_every")]
	pub execute_every: i64,
	pub name: String,
	pub heartbeat: Heartbeat,
	pub mysql: Option<Mysql>,
	pub redis: Option<Redis>
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Heartbeat {
	pub method: String,
	pub url: String
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mysql {
	pub url: String
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Redis {
	pub url: String
}
