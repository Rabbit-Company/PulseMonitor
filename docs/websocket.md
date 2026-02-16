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
         │ 5. {"action":"push","token":"..."}     │
         │───────────────────────────────────────>│
         │                                        │
         │ 6. {"action":"pushed",...}             │
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
	"latency": 45.123,
	"startTime": "2025-01-21T07:06:39.568Z",
	"endTime": "2025-01-21T07:06:39.613Z",
	"custom1": 12,
	"custom2": null,
	"custom3": null
}
```

| Field       | Type   | Description                        |
| ----------- | ------ | ---------------------------------- |
| `action`    | string | Always `"push"`                    |
| `token`     | string | Monitor's unique token             |
| `latency`   | number | Round-trip latency in milliseconds |
| `startTime` | string | Check start time (ISO 8601)        |
| `endTime`   | string | Check end time (ISO 8601)          |
| `custom1`   | number | Optional custom metric 1           |
| `custom2`   | number | Optional custom metric 2           |
| `custom3`   | number | Optional custom metric 3           |

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

Acknowledgment of received pulse:

```json
{
	"action": "pushed",
	"monitorId": "api-prod",
	"timestamp": "2025-01-21T07:06:39.700Z"
}
```

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
		"headers": []
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
		"custom1Oid": "1.3.6.1.4.1.2021.11.11.0",
		"custom2Oid": "1.3.6.1.4.1.2021.4.6.0",
		"custom3Oid": "1.3.6.1.4.1.2021.4.5.0"
	}
}
```

> **Note:** SNMP monitors populate `custom1`, `custom2`, and `custom3` in push messages with values from the configured OIDs.

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

### Heartbeat Delivery

When WebSocket is connected:

1. Pulses sent via WebSocket (preferred)
2. If WebSocket send fails → HTTP fallback

When WebSocket is disconnected:

1. Pulses sent via HTTP endpoint
2. Reconnection continues in background

### HTTP Fallback URL

When WebSocket is unavailable, pulses are sent to:

```
GET {PULSE_SERVER_URL}/v1/push/{token}?latency={latency}&startTime={startTimeISO}&endTime={endTimeISO}
```

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

### Common Issues

| Issue                                   | Cause               | Solution                                                |
| --------------------------------------- | ------------------- | ------------------------------------------------------- |
| "Authentication failed"                 | Invalid token       | Verify token matches server config                      |
| Frequent reconnects                     | Network instability | Check network, increase timeouts                        |
| Missing config updates                  | Token mismatch      | Ensure monitor's `pulseMonitors` includes this instance |
| "WebSocket pulse channel not available" | Disconnected        | Will auto-reconnect, HTTP fallback active               |
