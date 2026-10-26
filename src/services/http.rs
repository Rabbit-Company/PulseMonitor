use crate::utils::{CheckResult, Monitor};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use bytes::Bytes;
use futures_util::{StreamExt, stream::FuturesUnordered};
use hickory_resolver::{
	TokioResolver,
	config::{GOOGLE, LookupIpStrategy, ResolverConfig},
	net::runtime::TokioRuntimeProvider,
};
use http::{Request, Uri, header};
use http_body_util::{BodyExt, Full};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::{
	client::legacy::Client as HyperClient,
	rt::{TokioExecutor, TokioIo, TokioTimer},
};
use percent_encoding::percent_decode_str;
use reqwest::Client;
use std::{
	error::{Error, Error as StdError},
	future::Future,
	io,
	net::{IpAddr, SocketAddr},
	pin::Pin,
	sync::{Arc, OnceLock},
	task::{Context, Poll},
	time::{Duration, Instant},
};
use tokio::{net::TcpStream, time};
use tower_service::Service;
use tracing::{debug, warn};

const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const CONNECTION_STAGGER: Duration = Duration::from_millis(250);
const MIN_CONNECTION_STAGGER: Duration = Duration::from_millis(10);
const MAX_IN_FLIGHT_CONNECTS: usize = 4;

type RacingHttpsConnector = HttpsConnector<RacingConnector>;
type RacingClient = HyperClient<RacingHttpsConnector, Full<Bytes>>;
static RACING_CLIENT: OnceLock<RacingClient> = OnceLock::new();
static PROXY_CLIENT: OnceLock<Client> = OnceLock::new();
static DNS_RESOLVER: OnceLock<Arc<TokioResolver>> = OnceLock::new();

fn shared_resolver() -> Arc<TokioResolver> {
	DNS_RESOLVER
		.get_or_init(|| {
			let mut builder = TokioResolver::builder_tokio().unwrap_or_else(|error| {
				debug!("Failed to read system DNS configuration: {error}; using fallback resolver");
				TokioResolver::builder_with_config(
					ResolverConfig::udp_and_tcp(&GOOGLE),
					TokioRuntimeProvider::default(),
				)
			});
			builder.options_mut().ip_strategy = LookupIpStrategy::Ipv6AndIpv4;
			Arc::new(builder.build().expect("Failed to build HTTP DNS resolver"))
		})
		.clone()
}

#[derive(Clone)]
struct RacingConnector(Arc<TokioResolver>);

impl Service<Uri> for RacingConnector {
	type Response = TokioIo<TcpStream>;
	type Error = Box<dyn StdError + Send + Sync>;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, uri: Uri) -> Self::Future {
		let resolver = self.0.clone();
		Box::pin(async move {
			let host = uri.host().ok_or("HTTP URL is missing a host")?;
			let port = uri
				.port_u16()
				.unwrap_or(if uri.scheme_str() == Some("https") {
					443
				} else {
					80
				});
			let addresses = if let Ok(ip) = host
				.trim_start_matches('[')
				.trim_end_matches(']')
				.parse::<IpAddr>()
			{
				vec![SocketAddr::new(ip, port)]
			} else {
				resolver
					.lookup_ip(host)
					.await?
					.iter()
					.map(|ip| SocketAddr::new(ip, port))
					.collect()
			};
			Ok(TokioIo::new(race_tcp(addresses).await?))
		})
	}
}

// Alternate address families, then start another connect after 250 ms. A
// failed attempt immediately frees a slot; the first connected socket wins.
fn interleave_addresses(addresses: Vec<SocketAddr>) -> Vec<SocketAddr> {
	let prefer_v6 = addresses.first().is_some_and(SocketAddr::is_ipv6);
	let mut v4 = addresses.iter().copied().filter(SocketAddr::is_ipv4);
	let mut v6 = addresses.iter().copied().filter(SocketAddr::is_ipv6);
	let mut result = Vec::with_capacity(addresses.len());
	loop {
		let next = if prefer_v6 {
			v6.next().or_else(|| v4.next())
		} else {
			v4.next().or_else(|| v6.next())
		};
		let Some(first) = next else { break };
		result.push(first);
		if let Some(second) = if prefer_v6 { v4.next() } else { v6.next() } {
			result.push(second);
		}
	}
	result
}

async fn race_tcp(addresses: Vec<SocketAddr>) -> io::Result<TcpStream> {
	let socket = race_connect(addresses, |address| async move {
		match time::timeout(DEFAULT_CONNECT_TIMEOUT, TcpStream::connect(address)).await {
			Ok(result) => result,
			Err(_) => Err(io::Error::new(
				io::ErrorKind::TimedOut,
				format!("TCP connect to {address} timed out"),
			)),
		}
	})
	.await?;
	socket.set_nodelay(true)?;
	Ok(socket)
}

async fn race_connect<T, F, Fut>(addresses: Vec<SocketAddr>, connect: F) -> io::Result<T>
where
	T: Send + 'static,
	F: Fn(SocketAddr) -> Fut,
	Fut: Future<Output = io::Result<T>> + Send + 'static,
{
	let addresses = interleave_addresses(addresses);
	if addresses.is_empty() {
		return Err(io::Error::new(
			io::ErrorKind::AddrNotAvailable,
			"DNS returned no addresses",
		));
	}
	let mut pending: FuturesUnordered<Pin<Box<dyn Future<Output = io::Result<T>> + Send>>> =
		FuturesUnordered::new();
	let mut next = 0;
	let mut next_launch = time::Instant::now();
	let mut last_launch: Option<time::Instant> = None;
	let mut last_error = None;
	loop {
		tokio::select! {
			result = pending.next(), if !pending.is_empty() => {
				match result.expect("pending connect missing") {
					Ok(socket) => return Ok(socket),
					Err(error) => {
						last_error = Some(error);
						// Retry promptly without launching attempts less than 10 ms apart.
						next_launch = last_launch.map_or_else(time::Instant::now, |launched| {
							(launched + MIN_CONNECTION_STAGGER).max(time::Instant::now())
						});
					}
				}
			}
			_ = time::sleep_until(next_launch), if next < addresses.len() && pending.len() < MAX_IN_FLIGHT_CONNECTS => {
				let address = addresses[next];
				next += 1;
				last_launch = Some(time::Instant::now());
				pending.push(Box::pin(connect(address)));
				next_launch = last_launch.unwrap() + CONNECTION_STAGGER;
			}
		}
		if next == addresses.len() && pending.is_empty() {
			return Err(last_error.unwrap_or_else(|| io::Error::other("all TCP connections failed")));
		}
	}
}

fn racing_client() -> &'static RacingClient {
	RACING_CLIENT.get_or_init(|| {
		let connector = HttpsConnectorBuilder::new()
			.with_provider_and_platform_verifier(rustls::crypto::ring::default_provider())
			.expect("Failed to configure HTTPS certificate verifier")
			.https_or_http()
			.enable_http1()
			.wrap_connector(RacingConnector(shared_resolver()));
		HyperClient::builder(TokioExecutor::new())
			.pool_timer(TokioTimer::new())
			.pool_idle_timeout(Duration::from_secs(15))
			.pool_max_idle_per_host(2)
			.build(connector)
	})
}

fn uses_proxy(url: &str) -> bool {
	let scheme_proxy = if url
		.split_once(':')
		.is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("https"))
	{
		["HTTPS_PROXY", "https_proxy"]
	} else {
		["HTTP_PROXY", "http_proxy"]
	};
	scheme_proxy
		.into_iter()
		.chain(["ALL_PROXY", "all_proxy"])
		.any(|key| std::env::var_os(key).is_some_and(|value| !value.is_empty()))
}

async fn send_http(
	http: &crate::utils::HttpConfig,
	method: &str,
	want_body: bool,
	request_start: Instant,
) -> Result<(http::StatusCode, Option<String>, f64), Box<dyn Error + Send + Sync>> {
	let mut url = reqwest::Url::parse(&http.url)?;
	let url_auth = if !url.username().is_empty() || url.password().is_some() {
		let username = percent_decode_str(url.username())
			.decode_utf8()?
			.into_owned();
		let password = percent_decode_str(url.password().unwrap_or(""))
			.decode_utf8()?
			.into_owned();
		url.set_username("").map_err(|_| "Invalid URL username")?;
		url.set_password(None).map_err(|_| "Invalid URL password")?;
		Some(format!(
			"Basic {}",
			STANDARD.encode(format!("{username}:{password}"))
		))
	} else {
		None
	};
	let mut method = method.to_owned();
	let mut crossed_origin = false;
	for redirects in 0..=10 {
		url.set_fragment(None);
		let mut request = Request::builder().method(method.as_str()).uri(url.as_str());
		if !crossed_origin && let Some(auth) = &url_auth {
			request = request.header(header::AUTHORIZATION, auth.as_str());
		}
		if let Some(headers) = &http.headers {
			for header_map in headers {
				for (key, value) in header_map {
					// Do not send credentials from the original URL to another origin.
					if crossed_origin
						&& matches!(
							key.to_ascii_lowercase().as_str(),
							"authorization" | "cookie" | "proxy-authorization" | "host"
						) {
						continue;
					}
					request = request.header(key.as_str(), value.as_str());
				}
			}
		}
		let response = racing_client()
			.request(request.body(Full::new(Bytes::new()))?)
			.await?;
		let status = response.status();
		if status.is_redirection()
			&& let Some(location) = response.headers().get(header::LOCATION)
		{
			if redirects == 10 {
				return Err("Too many HTTP redirects".into());
			}
			let location = location.to_str()?;
			let next_url = url.join(location)?;
			if !matches!(next_url.scheme(), "http" | "https") {
				return Err("Redirect has an unsupported scheme".into());
			}
			crossed_origin |= next_url.origin() != url.origin();
			if status == http::StatusCode::SEE_OTHER
				|| (method == "POST"
					&& (status == http::StatusCode::MOVED_PERMANENTLY || status == http::StatusCode::FOUND))
			{
				method = "GET".to_owned();
			}
			url = next_url;
			continue;
		}
		let latency = request_start.elapsed().as_secs_f64() * 1000.0;
		let body = if want_body && status.is_success() {
			let bytes = response.into_body().collect().await?.to_bytes();
			Some(String::from_utf8_lossy(&bytes).into_owned())
		} else {
			None
		};
		return Ok((status, body, latency));
	}
	Err("Too many HTTP redirects".into())
}

async fn send_proxy_http(
	http: &crate::utils::HttpConfig,
	method: &str,
	want_body: bool,
	request_start: Instant,
) -> Result<(http::StatusCode, Option<String>, f64), Box<dyn Error + Send + Sync>> {
	let client = PROXY_CLIENT.get_or_init(|| {
		Client::builder()
			.pool_max_idle_per_host(2)
			.pool_idle_timeout(Duration::from_secs(15))
			.hickory_dns(true)
			.build()
			.expect("Failed to build proxy HTTP client")
	});
	let mut request = match method {
		"GET" => client.get(&http.url),
		"POST" => client.post(&http.url),
		"HEAD" => client.head(&http.url),
		_ => unreachable!(),
	};
	if let Some(headers) = &http.headers {
		for header_map in headers {
			for (key, value) in header_map {
				request = request.header(key, value);
			}
		}
	}
	let response = request.send().await?;
	let status = response.status();
	let latency = request_start.elapsed().as_secs_f64() * 1000.0;
	let body = if want_body && status.is_success() {
		Some(response.text().await?)
	} else {
		None
	};
	Ok((status, body, latency))
}

fn extract_json_value(json: &serde_json::Value, path: &str) -> Option<f64> {
	let mut current = json;

	for segment in path.split('.') {
		if segment.is_empty() {
			continue;
		}

		if let Some(index_str) = segment.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
			&& let Ok(index) = index_str.parse::<usize>()
		{
			current = current.get(index)?;
			continue;
		}

		current = current.get(segment)?;
	}

	match current {
		serde_json::Value::Number(n) => n.as_f64(),
		serde_json::Value::String(s) => s.trim().parse::<f64>().ok(),
		serde_json::Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
		_ => None,
	}
}

pub async fn is_http_online(
	monitor: &Monitor,
) -> Result<CheckResult, Box<dyn Error + Send + Sync>> {
	let http = monitor
		.http
		.as_ref()
		.ok_or("Monitor does not contain HTTP configuration")?;

	let timeout = Duration::from_secs(http.timeout.unwrap_or(DEFAULT_HTTP_TIMEOUT.as_secs()));
	let method = http.method.to_uppercase();
	if !matches!(method.as_str(), "GET" | "POST" | "HEAD") {
		return Err(format!("Unsupported HTTP method: {}", http.method).into());
	}
	let has_json_paths = http
		.json_paths
		.as_ref()
		.is_some_and(|paths| !paths.is_empty());

	let request_start = Instant::now();
	let (status, body, request_latency) = time::timeout(timeout, async {
		if uses_proxy(&http.url) {
			send_proxy_http(http, &method, has_json_paths, request_start).await
		} else {
			send_http(http, &method, has_json_paths, request_start).await
		}
	})
	.await??;

	if !status.is_success() {
		return Err(format!("Request failed with status: {status}").into());
	}

	let mut result = CheckResult::new();
	result.set("latency", request_latency);

	if has_json_paths {
		let body = body.expect("JSON body was not read");

		match serde_json::from_str::<serde_json::Value>(&body) {
			Ok(json) => {
				for (name, path) in http.json_paths.as_ref().unwrap() {
					match extract_json_value(&json, path) {
						Some(value) => {
							debug!("{} = {} (path: '{}')", name, value, path);
							result.set(name, value);
						}
						None => {
							warn!(
								"jsonPath '{}' ('{}') did not resolve to a numeric value",
								name, path
							);
						}
					}
				}
			}
			Err(e) => {
				warn!(
					"Failed to parse HTTP response as JSON for jsonPath extraction: {}",
					e
				);
			}
		}
	}

	Ok(result)
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;
	use std::sync::atomic::{AtomicUsize, Ordering};

	#[test]
	fn address_families_are_interleaved() {
		let addresses = vec![
			"[::1]:80".parse().unwrap(),
			"[::2]:80".parse().unwrap(),
			"127.0.0.1:80".parse().unwrap(),
			"127.0.0.2:80".parse().unwrap(),
		];
		let result = interleave_addresses(addresses);
		assert_eq!(result[0].to_string(), "[::1]:80");
		assert_eq!(result[1].to_string(), "127.0.0.1:80");
		assert_eq!(result[2].to_string(), "[::2]:80");
		assert_eq!(result[3].to_string(), "127.0.0.2:80");
	}

	#[tokio::test]
	async fn racing_connector_reaches_working_address_after_dead_addresses() {
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let working = listener.local_addr().unwrap();
		let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let dead = closed.local_addr().unwrap();
		drop(closed);
		let start = Instant::now();
		let socket = race_tcp(vec![dead, dead, working]).await.unwrap();
		assert_eq!(socket.peer_addr().unwrap(), working);
		assert!(start.elapsed() < Duration::from_secs(2));
	}

	#[tokio::test(start_paused = true)]
	async fn failed_connections_use_only_the_minimum_stagger() {
		let addresses = (1..=3)
			.map(|port| SocketAddr::from(([127, 0, 0, 1], port)))
			.collect();
		let start = time::Instant::now();
		let winner = race_connect(addresses, |address| async move {
			if address.port() < 3 {
				Err(io::ErrorKind::ConnectionRefused.into())
			} else {
				Ok(address)
			}
		})
		.await
		.unwrap();
		assert_eq!(winner.port(), 3);
		assert_eq!(start.elapsed(), MIN_CONNECTION_STAGGER * 2);
	}

	#[tokio::test]
	async fn stalled_addresses_do_not_consume_the_whole_check() {
		let addresses = (1..=31)
			.map(|port| SocketAddr::from(([127, 0, 0, 1], port)))
			.collect();
		let started = Arc::new(AtomicUsize::new(0));
		let start = Instant::now();
		let winner = race_connect(addresses, {
			let started = started.clone();
			move |address| {
				let started = started.clone();
				async move {
					started.fetch_add(1, Ordering::SeqCst);
					if address.port() <= 2 {
						std::future::pending().await
					} else {
						Ok(address)
					}
				}
			}
		})
		.await
		.unwrap();
		assert_eq!(winner.port(), 3);
		assert_eq!(started.load(Ordering::SeqCst), 3);
		assert!(start.elapsed() >= Duration::from_millis(450));
		assert!(start.elapsed() < Duration::from_secs(2));
	}

	#[tokio::test]
	async fn stalled_addresses_keep_connects_bounded() {
		let addresses = (1..=31)
			.map(|port| SocketAddr::from(([127, 0, 0, 1], port)))
			.collect();
		let started = Arc::new(AtomicUsize::new(0));
		let result = time::timeout(
			Duration::from_millis(1200),
			race_connect(addresses, {
				let started = started.clone();
				move |_address| {
					let started = started.clone();
					async move {
						started.fetch_add(1, Ordering::SeqCst);
						std::future::pending::<io::Result<SocketAddr>>().await
					}
				}
			}),
		)
		.await;
		assert!(result.is_err());
		assert_eq!(started.load(Ordering::SeqCst), MAX_IN_FLIGHT_CONNECTS);
	}

	#[tokio::test]
	async fn http_request_falls_back_through_dns_a_records() {
		use hickory_proto::{
			op::{Message, MessageType},
			rr::{RData, Record, RecordType, rdata::A},
		};
		use hickory_resolver::config::{ConnectionConfig, NameServerConfig};
		use tokio::io::{AsyncReadExt, AsyncWriteExt};

		let http_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let http_port = http_listener.local_addr().unwrap().port();
		let http_server = tokio::spawn(async move {
			let (mut stream, _) = http_listener.accept().await.unwrap();
			let mut request = [0; 1024];
			let count = stream.read(&mut request).await.unwrap();
			assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /health HTTP/1.1"));
			stream
				.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
				.await
				.unwrap();
		});

		let dns_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
		let dns_port = dns_socket.local_addr().unwrap().port();
		let dns_server = tokio::spawn(async move {
			let mut buffer = [0; 1500];
			loop {
				let (length, sender) = dns_socket.recv_from(&mut buffer).await.unwrap();
				let query = Message::from_vec(&buffer[..length]).unwrap();
				let question = query.queries.first().unwrap().clone();
				let mut reply = Message::new(query.id, MessageType::Response, query.op_code);
				reply.add_query(question.clone());
				if question.query_type() == RecordType::A {
					for last_octet in [2, 3, 1] {
						reply.add_answer(Record::from_rdata(
							question.name().clone(),
							60,
							RData::A(A::new(127, 0, 0, last_octet)),
						));
					}
				}
				dns_socket
					.send_to(&reply.to_vec().unwrap(), sender)
					.await
					.unwrap();
			}
		});

		let mut connection = ConnectionConfig::udp();
		connection.port = dns_port;
		let config = ResolverConfig::from_name_servers(vec![NameServerConfig::new(
			IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
			true,
			vec![connection],
		)]);
		let mut builder = TokioResolver::builder_with_config(config, TokioRuntimeProvider::default());
		builder.options_mut().ip_strategy = LookupIpStrategy::Ipv6AndIpv4;
		let resolver = Arc::new(builder.build().unwrap());
		let client: HyperClient<_, Full<Bytes>> =
			HyperClient::builder(TokioExecutor::new()).build(RacingConnector(resolver));
		let uri = format!("http://multi.test.:{http_port}/health");
		let request = Request::builder()
			.uri(uri)
			.body(Full::new(Bytes::new()))
			.unwrap();
		let response = time::timeout(Duration::from_secs(3), client.request(request))
			.await
			.unwrap()
			.unwrap();
		assert_eq!(response.status(), http::StatusCode::OK);
		http_server.await.unwrap();
		dns_server.abort();
	}

	#[tokio::test]
	async fn http_request_uses_one_connection_and_reads_json() {
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let server = tokio::spawn(async move {
			use tokio::io::{AsyncReadExt, AsyncWriteExt};
			let (mut stream, _) = listener.accept().await.unwrap();
			let mut request = Vec::new();
			let mut chunk = [0; 1024];
			while !request.windows(4).any(|window| window == b"\r\n\r\n") {
				let count = stream.read(&mut chunk).await.unwrap();
				assert!(count > 0);
				request.extend_from_slice(&chunk[..count]);
			}
			let request = String::from_utf8_lossy(&request);
			assert!(request.starts_with("GET /health HTTP/1.1"));
			assert!(request.contains("authorization: Basic dXNlcjpwYXNz\r\n"));
			stream
				.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 14\r\n\r\n{\"healthy\": 1}")
				.await
				.unwrap();
		});
		let config = crate::utils::HttpConfig {
			method: "GET".to_owned(),
			url: format!("http://user:pass@{address}/health#ignored"),
			timeout: Some(2),
			headers: None,
			json_paths: None,
		};
		let (status, body, _) = send_http(&config, "GET", true, Instant::now())
			.await
			.unwrap();
		assert_eq!(status, http::StatusCode::OK);
		assert_eq!(body.as_deref(), Some("{\"healthy\": 1}"));
		server.await.unwrap();
	}

	#[tokio::test]
	async fn http_post_follows_redirect_as_get() {
		use tokio::io::{AsyncReadExt, AsyncWriteExt};
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let server = tokio::spawn(async move {
			let (mut first, _) = listener.accept().await.unwrap();
			let mut request = [0; 1024];
			let count = first.read(&mut request).await.unwrap();
			assert!(String::from_utf8_lossy(&request[..count]).starts_with("POST /start HTTP/1.1"));
			first.write_all(b"HTTP/1.1 302 Found\r\nLocation: /target\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
			drop(first);
			let (mut second, _) = listener.accept().await.unwrap();
			let count = second.read(&mut request).await.unwrap();
			assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /target HTTP/1.1"));
			second
				.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
				.await
				.unwrap();
		});
		let config = crate::utils::HttpConfig {
			method: "POST".to_owned(),
			url: format!("http://{address}/start"),
			timeout: Some(2),
			headers: None,
			json_paths: None,
		};
		let (status, _, _) = time::timeout(
			Duration::from_secs(2),
			send_http(&config, "POST", false, Instant::now()),
		)
		.await
		.unwrap()
		.unwrap();
		assert_eq!(status, http::StatusCode::OK);
		server.await.unwrap();
	}

	#[tokio::test]
	async fn failed_status_does_not_wait_for_response_body() {
		use tokio::io::{AsyncReadExt, AsyncWriteExt};
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let server = tokio::spawn(async move {
			let (mut stream, _) = listener.accept().await.unwrap();
			let mut request = [0; 1024];
			let count = stream.read(&mut request).await.unwrap();
			assert!(count > 0);
			stream
				.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 100\r\n\r\n")
				.await
				.unwrap();
			time::sleep(Duration::from_secs(1)).await;
		});
		let config = crate::utils::HttpConfig {
			method: "GET".to_owned(),
			url: format!("http://{address}/"),
			timeout: Some(1),
			headers: None,
			json_paths: None,
		};
		let (status, body, _) = time::timeout(
			Duration::from_millis(300),
			send_http(&config, "GET", true, Instant::now()),
		)
		.await
		.unwrap()
		.unwrap();
		assert_eq!(status, http::StatusCode::SERVICE_UNAVAILABLE);
		assert!(body.is_none());
		server.abort();
	}

	#[tokio::test]
	async fn repeated_healthy_checks_reuse_one_connection() {
		use tokio::io::{AsyncReadExt, AsyncWriteExt};
		const CHECKS: usize = 1_000;
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let server = tokio::spawn(async move {
			let (mut stream, _) = listener.accept().await.unwrap();
			let mut chunk = [0; 1024];
			for _ in 0..CHECKS {
				let mut request = Vec::new();
				while !request.windows(4).any(|window| window == b"\r\n\r\n") {
					let count = stream.read(&mut chunk).await.unwrap();
					assert!(count > 0);
					request.extend_from_slice(&chunk[..count]);
				}
				stream
					.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
					.await
					.unwrap();
			}
		});
		let config = crate::utils::HttpConfig {
			method: "GET".to_owned(),
			url: format!("http://{address}/"),
			timeout: Some(2),
			headers: None,
			json_paths: None,
		};
		time::timeout(Duration::from_secs(10), async {
			for _ in 0..CHECKS {
				let (status, _, _) = send_http(&config, "GET", false, Instant::now())
					.await
					.unwrap();
				assert_eq!(status, http::StatusCode::OK);
			}
		})
		.await
		.unwrap();
		server.await.unwrap();
	}

	#[test]
	fn test_simple_object_path() {
		let json = json!({"cryptocurrencies": {"BTC": 32.543, "ETH": 32.432}});
		assert_eq!(
			extract_json_value(&json, "cryptocurrencies.BTC"),
			Some(32.543)
		);
		assert_eq!(
			extract_json_value(&json, "cryptocurrencies.ETH"),
			Some(32.432)
		);
	}

	#[test]
	fn test_array_index_path() {
		let json = json!({
			"system": {
				"cpu": [
					{"percentage": 85.5},
					{"percentage": 12.3}
				]
			}
		});
		assert_eq!(
			extract_json_value(&json, "system.cpu.[0].percentage"),
			Some(85.5)
		);
		assert_eq!(
			extract_json_value(&json, "system.cpu.[1].percentage"),
			Some(12.3)
		);
	}

	#[test]
	fn test_top_level_array() {
		let json = json!([10.0, 20.0, 30.0]);
		assert_eq!(extract_json_value(&json, "[0]"), Some(10.0));
		assert_eq!(extract_json_value(&json, "[2]"), Some(30.0));
	}

	#[test]
	fn test_nested_path() {
		let json = json!({"a": {"b": {"c": {"d": 42.0}}}});
		assert_eq!(extract_json_value(&json, "a.b.c.d"), Some(42.0));
	}

	#[test]
	fn test_string_numeric_value() {
		let json = json!({"value": "123.456"});
		assert_eq!(extract_json_value(&json, "value"), Some(123.456));
	}

	#[test]
	fn test_non_numeric_string_returns_none() {
		let json = json!({"value": "hello"});
		assert_eq!(extract_json_value(&json, "value"), None);
	}

	#[test]
	fn test_missing_path_returns_none() {
		let json = json!({"a": {"b": 1}});
		assert_eq!(extract_json_value(&json, "a.c"), None);
		assert_eq!(extract_json_value(&json, "x.y.z"), None);
	}

	#[test]
	fn test_array_out_of_bounds_returns_none() {
		let json = json!({"items": [1, 2, 3]});
		assert_eq!(extract_json_value(&json, "items.[99]"), None);
	}

	#[test]
	fn test_integer_value() {
		let json = json!({"count": 42});
		assert_eq!(extract_json_value(&json, "count"), Some(42.0));
	}

	#[test]
	fn test_boolean_value() {
		let json = json!({"active": true});
		assert_eq!(extract_json_value(&json, "active"), Some(1.0));
	}

	#[test]
	fn test_null_returns_none() {
		let json = json!({"value": null});
		assert_eq!(extract_json_value(&json, "value"), None);
	}
}
