use std::error::Error;
use tokio::time::{Duration, timeout};
use tokio_postgres::{Client, NoTls};

use crate::utils::Monitor;

pub async fn is_postgresql_online(monitor: &Monitor) -> Result<(), Box<dyn Error + Send + Sync>> {
	let postgresql = monitor
		.postgresql
		.as_ref()
		.ok_or("Monitor does not contain PostgreSQL configuration")?;

	let timeout_duration = Duration::from_secs(postgresql.timeout.unwrap_or(3));
	let use_tls = postgresql.use_tls.unwrap_or(false);

	if use_tls {
		let config = rustls::ClientConfig::builder()
			.with_root_certificates(rustls::RootCertStore::empty())
			.with_no_client_auth();
		let connector = tokio_postgres_rustls::MakeRustlsConnect::new(config);

		let (client, connection): (Client, _) = match timeout(timeout_duration, tokio_postgres::connect(&postgresql.url, connector)).await {
			Ok(Ok((client, connection))) => (client, connection),
			Ok(Err(e)) => return Err(Box::new(e)),
			Err(_) => return Err("PostgreSQL TLS connection timed out".into()),
		};

		tokio::spawn(connection);

		let query_result = timeout(timeout_duration, client.simple_query("SELECT 1")).await;

		match query_result {
			Ok(Ok(_)) => Ok(()),
			Ok(Err(e)) => Err(Box::new(e)),
			Err(_) => Err("PostgreSQL query timed out".into()),
		}
	} else {
		let (client, connection): (Client, _) = match timeout(timeout_duration, tokio_postgres::connect(&postgresql.url, NoTls)).await {
			Ok(Ok((client, connection))) => (client, connection),
			Ok(Err(e)) => return Err(Box::new(e)),
			Err(_) => return Err("PostgreSQL connection timed out".into()),
		};

		tokio::spawn(connection);

		let query_result = timeout(timeout_duration, client.simple_query("SELECT 1")).await;

		match query_result {
			Ok(Ok(_)) => Ok(()),
			Ok(Err(e)) => Err(Box::new(e)),
			Err(_) => Err("PostgreSQL query timed out".into()),
		}
	}
}