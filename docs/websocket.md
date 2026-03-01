# WebSocket Protocol

When running in WebSocket mode, PulseMonitor maintains a persistent connection to [UptimeMonitor-Server](https://github.com/Rabbit-Company/UptimeMonitor-Server) for configuration updates and pulse delivery.

## Connection Flow

```
┌─────────────────┐                    ┌─────────────────────┐
│  PulseMonitor   │                    │ UptimeMonitor-Server│
└────────┬────────┘                    └──────────┬──────────┘
         │                                        │
         │ 1. Connect to /ws                      │
         │───────────────────────────────────────>│
         │                                        │
         │ 2. {"action":"connected",...}          │
         │<───────────────────────────────────────│
         │                                        │
         │ 3. {"action":"subscribe","token":"..."}│
         │───────────────────────────────────────>│
         │                                        │
         │ 4. {"action":"subscribed","data":{...}}│
         │<───────────────────────────────────────│
         │                                        │
         │ 5. {"action":"push","pulseId":"..."}   │
         │───────────────────────────────────────>│
         │                                        │
         │ 6. {"action":"pushed","pulseId":"..."} │
         │<───────────────────────────────────────│
         │                                        │
         │ 7. {"action":"config-update",...}      │
         │<───────────────────────────────────────│ (on config change)
         │                                        │
```

## URL Construction

PulseMonitor converts the HTTP server URL to WebSocket:

| Server URL                   | WebSocket URL                 |
| ---------------------------- | ----------------------------- |
| `http://localhost:3000`      | `ws://localhost:3000/ws`      |
| `https://uptime.example.com` | `wss://uptime.example.com/ws` |
| `http://localhost:3000/`     | `ws://localhost:3000/ws`      |

## Message Types

### Client → Server

#### Subscribe

Authenticate and request monitor configuration:

```json
{
	"action": "subscribe",
	"token": "tk_pulse_us_west_1"
}
```

| Field    | Type   | Description                       |
| -------- | ------ | --------------------------------- |
| `action` | string | Always `"subscribe"`              |
| `token`  | string | PulseMonitor authentication token |

#### Push (Heartbeat)

Report successful monitor check:

```json
{
	"action": "push",
	"token": "tk_prod_api_abc123",
	"pulseId": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
	"latency": 123.456,
	"startTime": "2025-01-21T07:06:39.568Z",
	"endTime": "2025-01-21T07:06:39.691Z"
}
```

Push message with custom metrics (e.g. from Minecraft monitors):

```json
{
	"action": "push",
	"token": "tk_mc_server_abc123",
	"pulseId": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
	"latency": 45.123,
	"startTime": "2025-01-21T07:06:39.568Z",
	"endTime": "2025-01-21T07:06:39.613Z",
	"custom1": 12,
	"custom2": null,
	"custom3": null
}
```

| Field       | Type   | Description                                      |
| ----------- | ------ | ------------------------------------------------ |
| `action`    | string | Always `"push"`                                  |
| `token`     | string | Monitor's unique token                           |
| `pulseId`   | string | Unique identifier for this pulse (UUID, for ack) |
| `latency`   | number | Round-trip latency in milliseconds               |
| `startTime` | string | Check start time (ISO 8601)                      |
| `endTime`   | string | Check end time (ISO 8601)                        |
| `custom1`   | number | Optional custom metric 1                         |
| `custom2`   | number | Optional custom metric 2                         |
| `custom3`   | number | Optional custom metric 3                         |

### Server → Client

#### Connected

Initial connection acknowledgment:

```json
{
	"action": "connected",
	"message": "WebSocket connection established",
	"timestamp": "2025-01-21T07:06:00.000Z"
}
```

#### Subscribed

Successful authentication with initial configuration:

```json
{
	"action": "subscribed",
	"pulseMonitorId": "US-WEST-1",
	"pulseMonitorName": "US West 1 (Oregon)",
	"data": {
		"monitors": [
			{
				"enabled": true,
				"name": "Production API",
				"token": "tk_prod_api_abc123",
				"interval": 30,
				"http": {
					"method": "GET",
					"url": "https://api.example.com/health",
					"timeout": 10
				}
			}
		]
	},
	"timestamp": "2025-01-21T07:06:00.500Z"
}
```

| Field              | Type   | Description                    |
| ------------------ | ------ | ------------------------------ |
| `action`           | string | Always `"subscribed"`          |
| `pulseMonitorId`   | string | Assigned PulseMonitor ID       |
| `pulseMonitorName` | string | Human-readable name            |
| `data.monitors`    | array  | List of monitor configurations |
| `timestamp`        | string | Server timestamp               |

#### Config Update

Real-time configuration change:

```json
{
	"action": "config-update",
	"data": {
		"monitors": [
			{
				"enabled": true,
				"name": "Production API v2",
				"token": "tk_prod_api_abc123",
				"interval": 15,
				"http": {
					"method": "GET",
					"url": "https://api.example.com/v2/health",
					"timeout": 10
				}
			}
		]
	},
	"timestamp": "2025-01-21T08:00:00.000Z"
}
```

This message is sent when:

- Monitor configuration changes on the server
- Server reloads configuration via `/v1/reload/:token` endpoint
- Monitors are added/removed for this PulseMonitor

#### Pushed

Acknowledgment of a successfully stored pulse. The server echoes back the `pulseId` from the original push message so the client can match it to the queued pulse:

```json
{
	"action": "pushed",
	"pulseId": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
	"monitorId": "api-prod",
	"timestamp": "2025-01-21T07:06:39.700Z"
}
```

| Field       | Type   | Description                                                |
| ----------- | ------ | ---------------------------------------------------------- |
| `action`    | string | Always `"pushed"`                                          |
| `pulseId`   | string | The `pulseId` from the push message (null if not provided) |
| `monitorId` | string | Server-side monitor ID                                     |
| `timestamp` | string | Server timestamp                                           |

The client removes the pulse from its retry queue only after receiving a `pushed` message with the matching `pulseId`. If the server fails to store the pulse (example database is down), it responds with an `error` message instead, and the client will retry the pulse.

#### Error

Error response:

```json
{
	"action": "error",
	"message": "Invalid PulseMonitor token",
	"timestamp": "2025-01-21T07:06:00.100Z"
}
```

Common error messages:

- `"Invalid PulseMonitor token"` - Token not found in server config
- `"Invalid token"` - Monitor token invalid for push action
- `"Failed to store pulse"` - Server-side storage failure (client should retry)

## Pulse Delivery & Retry Queue

PulseMonitor uses a client-side retry queue to guarantee pulse delivery. This ensures that no monitoring data is lost during transient server outages, network failures, or WebSocket disconnects.

### How It Works

1. Each pulse is assigned a unique `pulseId` (UUID) before being sent
2. The pulse is stored in an in-memory queue until acknowledged
3. On receiving a `pushed` response with a matching `pulseId`, the pulse is removed from the queue
4. If no acknowledgment arrives within the retry delay, the pulse is resent
5. Pulses that exceed the maximum retry count are dropped

### Duplicate Prevention

Each queued pulse tracks when it was last sent. A pulse is only resent if the retry delay has fully elapsed since the last send attempt. This prevents duplicate deliveries while a pulse is in-flight awaiting acknowledgment.

### Configuration

| Environment Variable   | Default | Description                                      |
| ---------------------- | ------- | ------------------------------------------------ |
| `PULSE_MAX_QUEUE_SIZE` | `10000` | Maximum number of unacknowledged pulses in queue |
| `PULSE_MAX_RETRIES`    | `300`   | Maximum retry attempts before dropping a pulse   |
| `PULSE_RETRY_DELAY_MS` | `1000`  | Minimum delay between retry attempts (ms)        |

### Queue Behavior

The queue uses a HashMap + VecDeque hybrid structure for O(1) acknowledgment by `pulseId` and fair ordering. When the queue reaches capacity, the oldest pulse is evicted to make room for new ones.

The queue persists across WebSocket reconnections. If the connection drops and reconnects, unacknowledged pulses from the previous connection will be retried on the new connection.

### Capacity Planning

With default settings (max_retries=300, retry_delay=1000ms), each pulse can be retried for up to ~5 minutes. To estimate the queue size needed:

```
pulses_per_minute = monitor_count / average_interval_seconds × 60
queue_size_needed = pulses_per_minute × outage_minutes
```

Example: 1,000 monitors at 10s intervals during a 5-minute outage: `1000/10 × 60 × 5 = 30,000 pulses`. Set `PULSE_MAX_QUEUE_SIZE=50000` with headroom.

## Monitor Configuration Format

Monitors received via WebSocket use camelCase:

```json
{
	"enabled": true,
	"name": "Production API",
	"token": "tk_prod_api_abc123",
	"interval": 30,
	"debug": false,
	"http": {
		"method": "GET",
		"url": "https://api.example.com/health",
		"timeout": 10,
		"headers": [{ "Authorization": "Bearer token123" }]
	}
}
```

### Service Configuration Objects

Each monitor has one service type:

**HTTP:**

```json
{
	"http": {
		"method": "GET",
		"url": "https://example.com",
		"timeout": 10,
		"headers": [],
		"jsonPaths": {
			"custom1": "metrics.cpu",
			"custom2": "metrics.memory"
		}
	}
}
```

**WebSocket:**

```json
{
	"ws": {
		"url": "wss://example.com/socket",
		"timeout": 3
	}
}
```

**TCP:**

```json
{
	"tcp": {
		"host": "db.example.com",
		"port": 5432,
		"timeout": 5
	}
}
```

**UDP:**

```json
{
	"udp": {
		"host": "dns.example.com",
		"port": 53,
		"timeout": 3,
		"payload": "ping",
		"expectResponse": false
	}
}
```

**ICMP:**

```json
{
	"icmp": {
		"host": "8.8.8.8",
		"timeout": 2
	}
}
```

**SMTP:**

```json
{
	"smtp": {
		"url": "smtps://user:pass@mail.example.com:465"
	}
}
```

**IMAP:**

```json
{
	"imap": {
		"server": "imap.example.com",
		"port": 993,
		"username": "user@example.com",
		"password": "password"
	}
}
```

**MySQL:**

```json
{
	"mysql": {
		"url": "mysql://user:pass@localhost:3306/db",
		"timeout": 3
	}
}
```

**MSSQL:**

```json
{
	"mssql": {
		"url": "jdbc:sqlserver://localhost:1433;...",
		"timeout": 3
	}
}
```

**PostgreSQL:**

```json
{
	"postgresql": {
		"url": "postgresql://user:pass@localhost:5432/db",
		"timeout": 3,
		"useTls": false
	}
}
```

**Redis:**

```json
{
	"redis": {
		"url": "redis://user:pass@localhost:6379/0",
		"timeout": 3
	}
}
```

**SNMP:**

```json
{
	"snmp": {
		"host": "192.168.1.1",
		"port": 161,
		"timeout": 3,
		"version": "3",
		"community": "public",
		"username": "snmpv3user",
		"authPassword": "MyAuthPass",
		"authProtocol": "sha256",
		"privPassword": "MyPrivPass",
		"privCipher": "aes128",
		"securityLevel": "authPriv",
		"oid": "1.3.6.1.2.1.1.3.0",
		"oids": {
			"custom1": "1.3.6.1.4.1.2021.11.11.0",
			"custom2": "1.3.6.1.4.1.2021.4.6.0",
			"cpuIdle": "1.3.6.1.4.1.2021.11.11.0"
		}
	}
}
```

**Minecraft Java:**

```json
{
	"minecraft-java": {
		"host": "mc.example.com",
		"port": 25565,
		"timeout": 3
	}
}
```

**Minecraft Bedrock:**

```json
{
	"minecraft-bedrock": {
		"host": "bedrock.example.com",
		"port": 19132,
		"timeout": 3
	}
}
```

> **Note:** Minecraft Java and Bedrock monitors populate `custom1` in push messages with the current online player count.

## Connection Management

### Reconnection Behavior

PulseMonitor automatically reconnects on:

- Connection closed by server
- Network errors
- WebSocket protocol errors

Reconnection delay: **1 second** (constant)

```
Connection lost → Wait 1s → Reconnect → Subscribe → Resume monitoring
```

The pulse retry queue persists across reconnections. Unacknowledged pulses from the previous connection are retried on the new connection.

### Heartbeat Delivery

When WebSocket is connected:

1. Pulses are enqueued and sent via WebSocket
2. Each pulse is tracked by `pulseId` until acknowledged
3. If WebSocket send fails → HTTP fallback

When WebSocket is disconnected:

1. Pulses sent via HTTP endpoint with retries
2. Reconnection continues in background
3. Once reconnected, queued pulses are retried via WebSocket

### HTTP Fallback URL

When WebSocket is unavailable, pulses are sent to:

```
GET {PULSE_SERVER_URL}/v1/push/{token}?latency={latency}&startTime={startTimeISO}&endTime={endTimeISO}
```

The HTTP fallback uses its own retry loop with the same `PULSE_MAX_RETRIES` and `PULSE_RETRY_DELAY_MS` settings.

## Server Configuration

### UptimeMonitor-Server Setup

Define PulseMonitor instances in `config.toml`:

```toml
[[PulseMonitors]]
id = "US-WEST-1"
name = "US West 1 (Oregon)"
token = "tk_pulse_us_west_1"

[[PulseMonitors]]
id = "EU-CENTRAL-1"
name = "EU Central 1 (Frankfurt)"
token = "tk_pulse_eu_central_1"
```

### Assign Monitors to PulseMonitors

```toml
[[monitors]]
id = "api-prod"
name = "Production API"
token = "tk_prod_api_abc123"
interval = 30
pulseMonitors = ["US-WEST-1", "EU-CENTRAL-1"]

[monitors.pulse.http]
method = "GET"
url = "https://api.example.com/health"
timeout = 10
```

### Server-Side Requirements

The server must echo back the `pulseId` from push messages in the `pushed` response. This is required for the client-side retry queue to function correctly. If the server fails to store the pulse, it should respond with an `error` message instead of `pushed` so the client retries.

### Common Issues

| Issue                                   | Cause               | Solution                                                |
| --------------------------------------- | ------------------- | ------------------------------------------------------- |
| "Authentication failed"                 | Invalid token       | Verify token matches server config                      |
| Frequent reconnects                     | Network instability | Check network, increase timeouts                        |
| Missing config updates                  | Token mismatch      | Ensure monitor's `pulseMonitors` includes this instance |
| "WebSocket pulse channel not available" | Disconnected        | Will auto-reconnect, HTTP fallback active               |
| "Pulse queue full"                      | Prolonged outage    | Increase `PULSE_MAX_QUEUE_SIZE`                         |
