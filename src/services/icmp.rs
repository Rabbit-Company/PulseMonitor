use std::error::Error;

use crate::utils::{CheckResult, Monitor};

pub async fn is_icmp_online(
	monitor: &Monitor,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let icmp = monitor
		.icmp
		.as_ref()
		.ok_or("Monitor does not contain ICMP configuration")?;

	let timeout = icmp.timeout.unwrap_or(3);

	let output = tokio::process::Command::new("ping")
		.arg("-c")
		.arg("1")
		.arg("-W")
		.arg(timeout.to_string())
		.arg("-q")
		.arg(&icmp.host)
		.output()
		.await?;

	if output.status.success() {
		let stdout = String::from_utf8_lossy(&output.stdout);

		if let Some(rtt_line) = stdout.lines().last() {
			if let Some(values_part) = rtt_line.split('=').nth(1) {
				let parts: Vec<&str> = values_part.split('/').collect();

				if parts.len() >= 2 {
					if let Ok(avg) = parts[1].parse::<f64>() {
						return Ok(CheckResult {
							latency: Some(avg),
							custom1: None,
							custom2: None,
							custom3: None,
						});
					}
				}
			}
		}

		Ok(CheckResult::from_latency(None))
	} else {
		let stderr = String::from_utf8_lossy(&output.stderr);
		Err(format!("Ping to {} failed: {}", icmp.host, stderr).into())
	}
}
