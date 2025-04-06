use std::error::Error;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

use crate::utils::Monitor;

pub async fn is_tcp_online(monitor: &Monitor) -> Result<(), Box<dyn Error + Send + Sync>> {
	let tcp = monitor
		.tcp
		.as_ref()
		.ok_or("Monitor does not contain TCP configuration")?;

	let addr = format!("{}:{}", tcp.host, tcp.port);
	let timeout_duration = Duration::from_secs(tcp.timeout.unwrap_or(5));

	match timeout(timeout_duration, TcpStream::connect(&addr)).await {
		Ok(Ok(_stream)) => Ok(()),
		Ok(Err(e)) => Err(format!("Failed to connect to TCP server: {}", e).into()),
		Err(_) => Err("TCP connection attempt timed out".into()),
	}
}