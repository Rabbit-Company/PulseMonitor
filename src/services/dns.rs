use std::error::Error;
use std::net::SocketAddr;
use std::time::Instant;

use hickory_proto::op::{Message, ResponseCode};
use hickory_proto::rr::RecordType;
use hickory_proto::serialize::binary::{BinDecodable, BinEncodable};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket, lookup_host};
use tokio::time::{Duration, timeout};
use tracing::debug;

use crate::utils::{CheckResult, Monitor};

fn parse_record_type(s: &str) -> Result<RecordType, Box<dyn Error + Send + Sync>> {
	match s.to_uppercase().as_str() {
		"A" => Ok(RecordType::A),
		"AAAA" => Ok(RecordType::AAAA),
		"CAA" => Ok(RecordType::CAA),
		"CNAME" => Ok(RecordType::CNAME),
		"MX" => Ok(RecordType::MX),
		"NS" => Ok(RecordType::NS),
		"PTR" => Ok(RecordType::PTR),
		"SOA" => Ok(RecordType::SOA),
		"SRV" => Ok(RecordType::SRV),
		"TXT" => Ok(RecordType::TXT),
		"ANY" => Ok(RecordType::ANY),
		other => Err(format!(
			"Unsupported DNS record type '{}'. Supported: A, AAAA, CAA, CNAME, MX, NS, PTR, SOA, SRV, TXT, ANY",
			other
		)
		.into()),
	}
}

async fn resolve_target(
	host: &str,
	port: u16,
	timeout_dur: Duration,
) -> Result<SocketAddr, Box<dyn Error + Send + Sync>> {
	let host_port = format!("{}:{}", host, port);
	match timeout(timeout_dur, lookup_host(&host_port)).await {
		Ok(Ok(mut iter)) => iter
			.next()
			.ok_or_else(|| format!("Could not resolve DNS server host '{}'", host).into()),
		Ok(Err(e)) => Err(format!("Failed to resolve DNS server host '{}': {}", host, e).into()),
		Err(_) => Err(format!("Resolving DNS server host '{}' timed out", host).into()),
	}
}

/// Send a DNS query over UDP and return the raw response bytes.
async fn send_udp(
	addr: SocketAddr,
	payload: &[u8],
	timeout_dur: Duration,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
	let bind_addr = if addr.is_ipv4() {
		"0.0.0.0:0"
	} else {
		"[::]:0"
	};

	let socket = UdpSocket::bind(bind_addr).await?;
	socket.connect(addr).await?;

	match timeout(timeout_dur, socket.send(payload)).await {
		Ok(Ok(_)) => {}
		Ok(Err(e)) => return Err(format!("Failed to send DNS query: {}", e).into()),
		Err(_) => return Err("DNS query send timed out".into()),
	}

	let mut buf = vec![0u8; 4096];
	match timeout(timeout_dur, socket.recv(&mut buf)).await {
		Ok(Ok(n)) => {
			buf.truncate(n);
			Ok(buf)
		}
		Ok(Err(e)) => Err(format!("Failed to receive DNS response: {}", e).into()),
		Err(_) => Err("DNS response timed out".into()),
	}
}

/// Encode a DNS query message in RFC 1035 wire format.
fn build_dns_query(
	query_id: u16,
	name: &str,
	record_type: RecordType,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
	let mut buf = Vec::with_capacity(64);

	// Header
	buf.extend_from_slice(&query_id.to_be_bytes());
	buf.extend_from_slice(&[0x01, 0x00]); // flags: RD=1, everything else 0
	buf.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
	buf.extend_from_slice(&0u16.to_be_bytes()); // ANCOUNT
	buf.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
	buf.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT

	// Question: name as length-prefixed labels.
	for label in name.trim_end_matches('.').split('.') {
		if label.is_empty() {
			return Err(format!("Invalid DNS name '{}': empty label", name).into());
		}
		if label.len() > 63 {
			return Err(
				format!(
					"Invalid DNS name '{}': label '{}' exceeds 63 bytes",
					name, label
				)
				.into(),
			);
		}
		buf.push(label.len() as u8);
		buf.extend_from_slice(label.as_bytes());
	}
	buf.push(0); // root label terminator

	// QTYPE and QCLASS (IN = 1)
	let qtype: u16 = record_type.into();
	buf.extend_from_slice(&qtype.to_be_bytes());
	buf.extend_from_slice(&1u16.to_be_bytes());

	Ok(buf)
}

/// Send a DNS query over TCP (length-prefixed) and return the raw response bytes.
async fn send_tcp(
	addr: SocketAddr,
	payload: &[u8],
	timeout_dur: Duration,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
	let mut stream = match timeout(timeout_dur, TcpStream::connect(addr)).await {
		Ok(Ok(s)) => s,
		Ok(Err(e)) => return Err(format!("Failed to connect to DNS server over TCP: {}", e).into()),
		Err(_) => return Err("DNS TCP connection timed out".into()),
	};

	let len: u16 = payload
		.len()
		.try_into()
		.map_err(|_| "DNS query too large for TCP transport")?;
	let len_bytes = len.to_be_bytes();

	match timeout(timeout_dur, async {
		stream.write_all(&len_bytes).await?;
		stream.write_all(payload).await?;
		Ok::<_, std::io::Error>(())
	})
	.await
	{
		Ok(Ok(_)) => {}
		Ok(Err(e)) => return Err(format!("Failed to send DNS query over TCP: {}", e).into()),
		Err(_) => return Err("DNS TCP send timed out".into()),
	}

	let mut len_buf = [0u8; 2];
	match timeout(timeout_dur, stream.read_exact(&mut len_buf)).await {
		Ok(Ok(_)) => {}
		Ok(Err(e)) => return Err(format!("Failed to read DNS response length: {}", e).into()),
		Err(_) => return Err("DNS TCP response timed out".into()),
	}

	let resp_len = u16::from_be_bytes(len_buf) as usize;
	let mut buf = vec![0u8; resp_len];

	match timeout(timeout_dur, stream.read_exact(&mut buf)).await {
		Ok(Ok(_)) => Ok(buf),
		Ok(Err(e)) => Err(format!("Failed to read DNS TCP response body: {}", e).into()),
		Err(_) => Err("DNS TCP response body timed out".into()),
	}
}

pub async fn is_dns_online(monitor: &Monitor) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let dns = monitor
		.dns
		.as_ref()
		.ok_or("Monitor does not contain DNS configuration")?;

	let port = dns.port.unwrap_or(53);
	let timeout_dur = Duration::from_secs(dns.timeout.unwrap_or(3));
	let record_type_str = dns.record_type.as_deref().unwrap_or("A");
	let record_type = parse_record_type(record_type_str)?;
	let protocol = dns.protocol.as_deref().unwrap_or("udp").to_lowercase();
	let require_answer = dns.require_answer.unwrap_or(true);

	debug!(
		"DNS: querying {} {} via {}://{}:{}",
		dns.query, record_type_str, protocol, dns.host, port
	);

	let addr = resolve_target(&dns.host, port, timeout_dur).await?;

	let query_id: u16 = uuid::Uuid::new_v4().as_u128() as u16;

	let payload = build_dns_query(query_id, &dns.query, record_type)?;

	let payload = payload
		.to_bytes()
		.map_err(|e| format!("Failed to encode DNS query: {}", e))?;

	let start = Instant::now();

	let response_bytes = match protocol.as_str() {
		"udp" => send_udp(addr, &payload, timeout_dur).await?,
		"tcp" => send_tcp(addr, &payload, timeout_dur).await?,
		other => {
			return Err(format!("Unsupported DNS protocol '{}'. Use 'udp' or 'tcp'", other).into());
		}
	};

	let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

	let response = Message::from_bytes(&response_bytes)
		.map_err(|e| format!("Failed to parse DNS response: {}", e))?;

	if response.id != query_id {
		return Err(
			format!(
				"DNS response ID mismatch (sent {}, got {})",
				query_id, response.id
			)
			.into(),
		);
	}

	if response.response_code != ResponseCode::NoError {
		debug!(
			"DNS SERVFAIL: server={} query={} {} rcode={:?} raw_response_bytes={:02x?}",
			dns.host, dns.query, record_type_str, response.response_code, response_bytes,
		);
		return Err(
			format!(
				"DNS server returned error code: {:?}",
				response.response_code
			)
			.into(),
		);
	}

	let answer_count = response.answers.len() as usize;

	if require_answer && answer_count == 0 {
		return Err(
			format!(
				"DNS response for {} {} contained no answers",
				dns.query, record_type_str
			)
			.into(),
		);
	}

	if let Some(expected) = &dns.expected_value {
		let found = response
			.answers
			.iter()
			.any(|rec| rec.to_string().contains(expected));
		if !found {
			return Err(
				format!(
					"DNS response did not contain expected value '{}' in any answer record",
					expected
				)
				.into(),
			);
		}
	}

	debug!(
		"DNS: {} {} returned {} answer(s) in {:.3}ms",
		dns.query, record_type_str, answer_count, latency_ms
	);

	let mut result = CheckResult::new();
	result.set("latency", latency_ms);
	result.set("custom1", answer_count as f64);
	result.set("answerCount", answer_count as f64);

	Ok(result)
}
