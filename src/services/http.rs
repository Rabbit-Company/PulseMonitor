use crate::utils::{CheckResult, Monitor};
use reqwest::Client;
use std::{
	error::Error,
	sync::OnceLock,
	time::{Duration, Instant},
};
use tracing::{debug, warn};

fn shared_client() -> &'static Client {
	static CLIENT: OnceLock<Client> = OnceLock::new();
	CLIENT.get_or_init(|| {
		Client::builder()
			.pool_max_idle_per_host(256)
			.build()
			.expect("Failed to build HTTP client")
	})
}

fn extract_json_value(json: &serde_json::Value, path: &str) -> Option<f64> {
	let mut current = json;

	for segment in path.split('.') {
		if segment.is_empty() {
			continue;
		}

		if let Some(index_str) = segment.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
			if let Ok(index) = index_str.parse::<usize>() {
				current = current.get(index)?;
				continue;
			}
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

	let client = shared_client();
	let timeout = Duration::from_secs(http.timeout.unwrap_or(10));

	let mut request = match http.method.to_uppercase().as_str() {
		"GET" => client.get(&http.url).timeout(timeout),
		"POST" => client.post(&http.url).timeout(timeout),
		"HEAD" => client.head(&http.url).timeout(timeout),
		_ => return Err(format!("Unsupported HTTP method: {}", http.method).into()),
	};

	if let Some(headers) = &http.headers {
		for header in headers {
			for (key, value) in header {
				request = request.header(key, value);
			}
		}
	}

	let has_json_paths = http
		.json_paths
		.as_ref()
		.is_some_and(|paths| !paths.is_empty());

	let request_start = Instant::now();
	let response = request.send().await?;
	let request_latency = request_start.elapsed().as_secs_f64() * 1000.0;

	if !response.status().is_success() {
		return Err(format!("Request failed with status: {}", response.status()).into());
	}

	let mut result = CheckResult::new();
	result.set("latency", request_latency);

	if has_json_paths {
		let body = response.text().await?;

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
