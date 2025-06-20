use std::error::Error;

use crate::utils::Monitor;

pub async fn is_icmp_online(monitor: &Monitor) -> Result<(), Box<dyn Error + Send + Sync>> {
	let icmp = monitor
		.icmp
		.as_ref()
		.ok_or("Monitor does not contain ICMP configuration")?;

	let timeout = icmp.timeout.unwrap_or(3);

	let output = tokio::process::Command::new("ping")
		.arg("-c").arg("1") // Send 1 packet
		.arg("-W").arg(timeout.to_string()) // Timeout in seconds
		.arg("-q") // Quiet mode
		.arg(&icmp.host)
		.output()
		.await?;

	if output.status.success() {
    Ok(())
  } else {
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("Ping to {} failed: {}", icmp.host, stderr).into())
  }
}
