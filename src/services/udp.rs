use std::error::Error;
use tokio::net::UdpSocket;
use tokio::time::{Duration, timeout};

use crate::utils::{CheckResult, Monitor};

pub async fn is_udp_online(monitor: &Monitor) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let udp = monitor
		.udp
		.as_ref()
		.ok_or("Monitor does not contain UDP configuration")?;

	let target = format!("{}:{}", udp.host, udp.port);
	let timeout_duration = Duration::from_secs(udp.timeout.unwrap_or(3));

	let socket = UdpSocket::bind("0.0.0.0:0").await?;
	let message = udp.payload.as_deref().unwrap_or("ping");

	socket.send_to(message.as_bytes(), &target).await?;

	if udp.expect_response.unwrap_or(false) {
		let mut buf = [0u8; 1024];
		match timeout(timeout_duration, socket.recv_from(&mut buf)).await {
			Ok(Ok((_n, _src))) => Ok(CheckResult::from_latency(None)),
			Ok(Err(e)) => Err(format!("Failed to receive UDP response: {}", e).into()),
			Err(_) => Err("UDP response timed out".into()),
		}
	} else {
		Ok(CheckResult::from_latency(None))
	}
}
