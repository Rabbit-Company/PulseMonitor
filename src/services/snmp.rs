use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use snmp2::{Oid, SyncSession, Value, v3};
use tracing::{debug, error};

use crate::utils::{CheckResult, Monitor};

/// Parse a dot-notation OID string (e.g., "1.3.6.1.2.1.1.3.0") into an Oid.
fn parse_oid(oid_str: &str) -> Result<Oid<'static>, Box<dyn Error + Send + Sync>> {
	let parts: Vec<u64> = oid_str
		.trim_matches('.')
		.split('.')
		.map(|s| {
			s.parse::<u64>()
				.map_err(|e| format!("Invalid OID component '{}': {}", s, e))
		})
		.collect::<Result<Vec<_>, _>>()?;

	Oid::from(parts.as_slice())
		.map_err(|e| format!("Failed to create OID from '{}': {:?}", oid_str, e).into())
}

/// Convert an SNMP Value to f64 for use as a custom metric.
fn value_to_f64(value: &Value) -> Option<f64> {
	match value {
		Value::Integer(v) => Some(*v as f64),
		Value::Counter32(v) => Some(*v as f64),
		Value::Unsigned32(v) => Some(*v as f64),
		Value::Timeticks(v) => Some(*v as f64),
		Value::Counter64(v) => Some(*v as f64),
		Value::OctetString(bytes) => std::str::from_utf8(bytes)
			.ok()
			.and_then(|s| s.trim().parse::<f64>().ok()),
		_ => None,
	}
}

/// Parse the authentication protocol string from config.
fn parse_auth_protocol(proto: &str) -> Result<v3::AuthProtocol, Box<dyn Error + Send + Sync>> {
	match proto.to_lowercase().as_str() {
		"md5" => Ok(v3::AuthProtocol::Md5),
		"sha" | "sha1" | "sha-1" => Ok(v3::AuthProtocol::Sha1),
		"sha224" | "sha-224" => Ok(v3::AuthProtocol::Sha224),
		"sha256" | "sha-256" => Ok(v3::AuthProtocol::Sha256),
		"sha384" | "sha-384" => Ok(v3::AuthProtocol::Sha384),
		"sha512" | "sha-512" => Ok(v3::AuthProtocol::Sha512),
		_ => Err(
			format!(
				"Unsupported auth protocol '{}'. Supported: md5, sha1, sha224, sha256, sha384, sha512",
				proto
			)
			.into(),
		),
	}
}

/// Parse the privacy cipher string from config.
fn parse_priv_cipher(cipher: &str) -> Result<v3::Cipher, Box<dyn Error + Send + Sync>> {
	match cipher.to_lowercase().as_str() {
		"des" => Ok(v3::Cipher::Des),
		"aes" | "aes128" | "aes-128" => Ok(v3::Cipher::Aes128),
		"aes192" | "aes-192" => Ok(v3::Cipher::Aes192),
		"aes256" | "aes-256" => Ok(v3::Cipher::Aes256),
		_ => Err(
			format!(
				"Unsupported privacy cipher '{}'. Supported: des, aes128, aes192, aes256",
				cipher
			)
			.into(),
		),
	}
}

/// Query named OIDs and populate values in the CheckResult.
fn query_named_oids(
	session: &mut SyncSession,
	oids: &HashMap<String, String>,
	result: &mut CheckResult,
) {
	for (name, oid_str) in oids {
		let oid = match parse_oid(oid_str) {
			Ok(o) => o,
			Err(e) => {
				error!(
					"SNMP: failed to parse OID '{}' for '{}': {}",
					oid_str, name, e
				);
				continue;
			}
		};

		debug!("SNMP: querying '{}' OID '{}'", name, oid_str);

		let mut retries = 3;
		let value = loop {
			match session.get(&oid) {
				Ok(response) => {
					break response
						.varbinds
						.into_iter()
						.next()
						.and_then(|(_oid, val)| value_to_f64(&val));
				}
				Err(snmp2::Error::AuthUpdated) if retries > 0 => {
					retries -= 1;
					continue;
				}
				Err(e) => {
					error!("SNMP: '{}' OID query failed: {}", name, e);
					break None;
				}
			}
		};

		if let Some(v) = value {
			debug!("SNMP: '{}' = {}", name, v);
			result.set(name, v);
		} else {
			debug!("SNMP: '{}' returned no numeric value", name);
		}
	}
}

/// Format the SNMP target address, handling IPv6 correctly.
fn format_addr(host: &str, port: u16) -> String {
	if host.contains(':') && !host.starts_with('[') {
		format!("[{}]:{}", host, port)
	} else {
		format!("{}:{}", host, port)
	}
}

/// Perform an SNMP GET with retry on AuthUpdated errors.
fn snmp_get_with_retry(
	session: &mut SyncSession,
	oid: &Oid<'_>,
	oid_str: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let mut retries = 3;
	loop {
		match session.get(oid) {
			Ok(_response) => {
				debug!("SNMP: primary OID query succeeded");
				return Ok(());
			}
			Err(snmp2::Error::AuthUpdated) if retries > 0 => {
				debug!("SNMP: AuthUpdated, retrying ({} left)", retries);
				retries -= 1;
				continue;
			}
			Err(e) => {
				return Err(format!("SNMP GET for OID '{}' failed: {}", oid_str, e).into());
			}
		}
	}
}

/// Run SNMPv1 check.
fn run_snmp_v1(
	addr: &str,
	timeout: Duration,
	community: &[u8],
	oid_str: &str,
	oids: &HashMap<String, String>,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	debug!("SNMP: creating v1 session to {}", addr);

	let mut session = SyncSession::new_v1(addr, community, Some(timeout), 0)
		.map_err(|e| format!("Failed to create SNMPv1 session to {}: {}", addr, e))?;

	debug!("SNMP: v1 session created, querying OID '{}'", oid_str);

	let primary_oid = parse_oid(oid_str)?;
	snmp_get_with_retry(&mut session, &primary_oid, oid_str)?;

	let mut result = CheckResult::new();
	query_named_oids(&mut session, oids, &mut result);

	debug!("SNMP v1: done. values={:?}", result.values);

	Ok(result)
}

/// Run SNMPv2c check.
fn run_snmp_v2c(
	addr: &str,
	timeout: Duration,
	community: &[u8],
	oid_str: &str,
	oids: &HashMap<String, String>,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	debug!("SNMP: creating v2c session to {}", addr);

	let mut session = SyncSession::new_v2c(addr, community, Some(timeout), 0)
		.map_err(|e| format!("Failed to create SNMPv2c session to {}: {}", addr, e))?;

	debug!("SNMP: v2c session created, querying OID '{}'", oid_str);

	let primary_oid = parse_oid(oid_str)?;
	snmp_get_with_retry(&mut session, &primary_oid, oid_str)?;

	let mut result = CheckResult::new();
	query_named_oids(&mut session, oids, &mut result);

	debug!("SNMP v2c: done. values={:?}", result.values);

	Ok(result)
}

/// Run SNMPv3 check.
fn run_snmp_v3(
	addr: &str,
	timeout: Duration,
	username: &str,
	auth_password: &str,
	auth_protocol: &str,
	security_level: &str,
	priv_password: Option<String>,
	priv_cipher: &str,
	oid_str: &str,
	oids: &HashMap<String, String>,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let auth_proto = parse_auth_protocol(auth_protocol)?;

	debug!("SNMP: building v3 security params for {}", addr);

	let security = {
		let base = v3::Security::new(username.as_bytes(), auth_password.as_bytes())
			.with_auth_protocol(auth_proto);

		match security_level.to_lowercase().as_str() {
			"noauthnopriv" => base,
			"authnopriv" => base.with_auth(v3::Auth::AuthNoPriv),
			"authpriv" => {
				let cipher = parse_priv_cipher(priv_cipher)?;
				let priv_pass = priv_password.unwrap_or_default();
				base.with_auth(v3::Auth::AuthPriv {
					cipher,
					privacy_password: priv_pass.into_bytes(),
				})
			}
			_ => {
				return Err(
					format!(
						"Unsupported security level '{}'. Supported: noAuthNoPriv, authNoPriv, authPriv",
						security_level
					)
					.into(),
				);
			}
		}
	};

	debug!("SNMP: creating v3 session to {}", addr);

	let mut session = SyncSession::new_v3(addr, Some(timeout), 0, security)
		.map_err(|e| format!("Failed to create SNMPv3 session to {}: {}", addr, e))?;

	debug!("SNMP: v3 session created, calling init()");

	let init_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| session.init()));
	match init_result {
		Ok(Ok(())) => debug!("SNMP: init() succeeded"),
		Ok(Err(e)) => return Err(format!("SNMPv3 session init failed for {}: {}", addr, e).into()),
		Err(_) => {
			return Err(
				format!(
					"SNMPv3 init panicked for {} (target may be unreachable)",
					addr
				)
				.into(),
			);
		}
	}

	debug!("SNMP: querying primary OID '{}'", oid_str);

	let primary_oid = parse_oid(oid_str)?;
	snmp_get_with_retry(&mut session, &primary_oid, oid_str)?;

	let mut result = CheckResult::new();
	query_named_oids(&mut session, oids, &mut result);

	debug!("SNMP v3: done. values={:?}", result.values);

	Ok(result)
}

pub async fn is_snmp_online(
	monitor: &Monitor,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let snmp = monitor
		.snmp
		.as_ref()
		.ok_or("Monitor does not contain SNMP configuration")?;

	let host = snmp.host.clone();
	let port = snmp.port.unwrap_or(161);
	let timeout_secs = snmp.timeout.unwrap_or(3);
	let version = snmp.version.clone().unwrap_or_else(|| "3".to_string());
	let community = snmp
		.community
		.clone()
		.unwrap_or_else(|| "public".to_string());
	let username = snmp.username.clone().unwrap_or_default();
	let auth_password = snmp.auth_password.clone().unwrap_or_default();
	let auth_protocol = snmp
		.auth_protocol
		.clone()
		.unwrap_or_else(|| "sha256".to_string());
	let priv_password = snmp.priv_password.clone();
	let priv_cipher = snmp
		.priv_cipher
		.clone()
		.unwrap_or_else(|| "aes128".to_string());
	let security_level = snmp
		.security_level
		.clone()
		.unwrap_or_else(|| "authPriv".to_string());
	let oid = snmp
		.oid
		.clone()
		.unwrap_or_else(|| "1.3.6.1.2.1.1.3.0".to_string());

	let oids: HashMap<String, String> = snmp.oids.clone().unwrap_or_default();

	debug!(
		"SNMP: connecting to {}:{} version={}, user='{}', oids={:?}",
		host, port, version, username, oids
	);

	tokio::task::spawn_blocking(move || {
		let addr = format_addr(&host, port);
		let timeout = Duration::from_secs(timeout_secs);

		match version.as_str() {
			"1" | "v1" => run_snmp_v1(&addr, timeout, community.as_bytes(), &oid, &oids),
			"2" | "2c" | "v2" | "v2c" => run_snmp_v2c(&addr, timeout, community.as_bytes(), &oid, &oids),
			"3" | "v3" => run_snmp_v3(
				&addr,
				timeout,
				&username,
				&auth_password,
				&auth_protocol,
				&security_level,
				priv_password,
				&priv_cipher,
				&oid,
				&oids,
			),
			_ => Err(
				format!(
					"Unsupported SNMP version '{}'. Supported: 1, 2c, 3",
					version
				)
				.into(),
			),
		}
	})
	.await
	.map_err(|e| -> Box<dyn Error + Send + Sync> { format!("SNMP task panicked: {}", e).into() })?
}
