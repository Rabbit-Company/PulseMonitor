use crate::utils::{CheckResult, Monitor};
use std::error::Error;
use std::time::Duration;
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

pub async fn is_mssql_online(
	monitor: &Monitor,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let mssql = monitor
		.mssql
		.as_ref()
		.ok_or("Monitor does not contain MSSQL configuration")?;

	let timeout: Duration = Duration::from_secs(mssql.timeout.unwrap_or(3));

	let config: Config = Config::from_jdbc_string(&mssql.url)?;

	let tcp = tokio::time::timeout(timeout, TcpStream::connect(config.get_addr()))
		.await
		.map_err(|_| "TCP connection to MSSQL timed out")??;

	tcp.set_nodelay(true)?;

	let mut client = tokio::time::timeout(timeout, Client::connect(config, tcp.compat_write()))
		.await
		.map_err(|_| "MSSQL handshake timed out")??;

	client.query("SELECT 1", &[]).await?;

	Ok(CheckResult::from_latency(None))
}
