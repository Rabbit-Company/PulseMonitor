use std::error::Error;

use lettre::{SmtpTransport, Transport};

use crate::utils::Monitor;

pub async fn is_smtp_online(monitor: &Monitor) -> Result<(), Box<dyn Error + Send + Sync>> {
	let smtp = monitor
		.smtp
		.as_ref()
		.ok_or("Monitor does not contain SMTP configuration")?;

	let sender = SmtpTransport::from_url(&smtp.url)?.build();
	sender.test_connection()?;
	sender.shutdown();

	Ok(())
}