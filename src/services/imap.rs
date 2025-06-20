use std::error::Error;

use crate::utils::Monitor;

pub async fn is_imap_online(
	monitor: &Monitor,
) -> Result<Option<f64>, Box<dyn Error + Send + Sync>> {
	let imap = monitor
		.imap
		.as_ref()
		.ok_or("Monitor does not contain IMAP configuration")?;

	let client = imap::ClientBuilder::new(&imap.server, imap.port).connect()?;

	let mut imap_session = client
		.login(&imap.username, &imap.password)
		.map_err(|e| e.0)?;

	imap_session.logout()?;

	Ok(None)
}
