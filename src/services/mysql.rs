use mysql_async::{prelude::Queryable, Opts, OptsBuilder, Pool};
use std::error::Error;
use tokio::time::Duration;

use crate::utils::Monitor;

pub async fn is_mysql_online(monitor: &Monitor) -> Result<(), Box<dyn Error + Send + Sync>> {
	let mysql = monitor
		.mysql
		.as_ref()
		.ok_or("Monitor does not contain MYSQL configuration")?;

	let timeout: Duration = Duration::from_secs(mysql.timeout.unwrap_or(3));

	let opts = Opts::from_url(&mysql.url)?;
	let builder = OptsBuilder::from_opts(opts)
		.conn_ttl(timeout)
		.stmt_cache_size(0);

	let pool = Pool::new(builder);

	let conn_result = tokio::time::timeout(timeout, pool.get_conn()).await;

	let mut conn = match conn_result {
		Ok(Ok(conn)) => conn,
		Ok(Err(e)) => return Err(Box::new(e)),
		Err(_) => return Err("MySQL connection timed out".into()),
	};

	let ping_result = tokio::time::timeout(timeout, conn.query_drop("SELECT 1")).await;

	match ping_result {
		Ok(Ok(_)) => Ok(()),
		Ok(Err(e)) => Err(Box::new(e)),
		Err(_) => Err("MySQL query timed out".into()),
	}
}
