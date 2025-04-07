use std::error::Error;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;
use ping::dgramsock::ping;

use crate::utils::Monitor;

pub async fn is_icmp_online(monitor: &Monitor) -> Result<(), Box<dyn Error + Send + Sync>> {
	let icmp = monitor
		.icmp
		.as_ref()
		.ok_or("Monitor does not contain ICMP configuration")?;

	let addr = IpAddr::from_str(&icmp.host)?;
	let timeout = icmp.timeout.unwrap_or(3);

	let result = ping(addr, Some(Duration::from_secs(timeout)), None, None, None, None);

	match result {
		Ok(()) => Ok(()),
		Err(e) => {
			Err(format!("Ping to {} failed: {}", icmp.host, e).into())
		},
	}
}