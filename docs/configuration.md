# Configuration Guide

PulseMonitor can be configured via a `config.toml` file (file mode) or receive configuration from [UptimeMonitor-Server](https://github.com/Rabbit-Company/UptimeMonitor-Server) (WebSocket mode).

## Quick Start

Minimal configuration to monitor an HTTP endpoint:

```toml
[[monitors]]
enabled = true
name = "My Website"
interval = 30

[monitors.heartbeat]
method = "GET"
url = "https://uptime.example.com/api/push/YOUR_TOKEN?latency={latency}"

[monitors.http]
method = "GET"
url = "https://example.com"
timeout = 10
```

## Monitor Structure

Every monitor follows this structure:

```toml
[[monitors]]
enabled = true           # Enable/disable this monitor
name = "Monitor Name"    # Display name for logging
interval = 30            # Check interval in seconds
debug = false            # Enable verbose logging (optional)

[monitors.heartbeat]     # Where to send success notifications
# ... heartbeat config

[monitors.SERVICE]       # One of: http, ws, tcp, udp, icmp, smtp, imap, mysql, mssql, postgresql, redis
# ... service-specific config
```

## Common Options

| Option     | Type    | Default | Description                    |
| ---------- | ------- | ------- | ------------------------------ |
| `enabled`  | boolean | -       | Whether this monitor is active |
| `name`     | string  | -       | Display name for logging       |
| `interval` | integer | -       | Seconds between checks         |
| `debug`    | boolean | `false` | Enable verbose logging         |

## Heartbeat Configuration

The heartbeat section defines where to send success notifications:

```toml
[monitors.heartbeat]
method = "GET"           # HTTP method: GET, POST, HEAD
timeout = 10             # Request timeout in seconds
url = "https://uptime.example.com/api/push/TOKEN?latency={latency}"
headers = [
  { "Authorization" = "Bearer YOUR_TOKEN" },
  { "X-Latency" = "{latency}" }
]
```

### Template Placeholders

Use these variables in `url` and `headers`:

| Placeholder       | Description             | Example Output             |
| ----------------- | ----------------------- | -------------------------- |
| `{latency}`       | Round-trip latency (ms) | `123.456`                  |
| `{startTimeISO}`  | Check start (ISO 8601)  | `2025-01-21T07:06:39.568Z` |
| `{endTimeISO}`    | Check end (ISO 8601)    | `2025-01-21T07:06:40.000Z` |
| `{startTimeUnix}` | Check start (Unix ms)   | `1753081599568`            |
| `{endTimeUnix}`   | Check end (Unix ms)     | `1753081600000`            |

### Example with All Placeholders

```toml
[monitors.heartbeat]
method = "POST"
timeout = 10
url = "https://api.example.com/heartbeat?latency={latency}&start={startTimeISO}&end={endTimeISO}"
headers = [
  { "Content-Type" = "application/json" },
  { "X-Latency" = "{latency}" },
  { "X-Start-Time" = "{startTimeUnix}" },
  { "X-End-Time" = "{endTimeUnix}" }
]
```

## Service Configurations

### HTTP Monitoring

```toml
[monitors.http]
method = "GET"           # GET, POST, HEAD
url = "https://api.example.com/health"
timeout = 10             # Seconds (default: 10)
headers = [
  { "Authorization" = "Bearer TOKEN" },
  { "User-Agent" = "PulseMonitor/3.12" }
]
```

### WebSocket Monitoring

```toml
[monitors.ws]
url = "ws://example.com/socket"    # ws:// or wss://
timeout = 3                         # Seconds (default: 3)
```

### TCP Monitoring

```toml
[monitors.tcp]
host = "127.0.0.1"
port = 8080
timeout = 5              # Seconds (default: 5)
```

### UDP Monitoring

```toml
[monitors.udp]
host = "127.0.0.1"
port = 9000
timeout = 3              # Seconds (default: 3)
payload = "ping"         # Data to send (default: "ping")
expect_response = true   # Wait for response (default: false)
```

### ICMP (Ping) Monitoring

```toml
[monitors.icmp]
host = "8.8.8.8"
timeout = 2              # Seconds (default: 3)
```

> **Note:** ICMP requires root/administrator privileges or `CAP_NET_RAW` capability.

### SMTP Monitoring

```toml
[monitors.smtp]
# SMTP over TLS (recommended)
url = "smtps://user:pass@mail.example.com:465"

# SMTP with STARTTLS
# url = "smtp://user:pass@mail.example.com:587?tls=required"

# Unencrypted SMTP (not recommended)
# url = "smtp://user:pass@mail.example.com:25"
```

### IMAP Monitoring

```toml
[monitors.imap]
server = "imap.example.com"
port = 993
username = "user@example.com"
password = "your_password"
```

### MySQL Monitoring

```toml
[monitors.mysql]
url = "mysql://username:password@localhost:3306/database?require_ssl=true"
timeout = 3              # Seconds (default: 3)
```

### MSSQL Monitoring

```toml
[monitors.mssql]
url = "jdbc:sqlserver://localhost:1433;databaseName=master;encrypt=true;user=sa;password=Password123;TrustServerCertificate=true;"
timeout = 3              # Seconds (default: 3)
```

### PostgreSQL Monitoring

```toml
[monitors.postgresql]
url = "postgresql://username:password@localhost:5432/database"
timeout = 3              # Seconds (default: 3)
use_tls = false          # Enable TLS (default: false)
```

### Redis Monitoring

```toml
[monitors.redis]
url = "redis://username:password@localhost:6379/0"
timeout = 3              # Seconds (default: 3)
```

## Complete Configuration Example

```toml
# HTTP API Monitor
[[monitors]]
enabled = true
name = "Production API"
interval = 30
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://uptime.example.com/api/push/api-token?latency={latency}"

[monitors.http]
method = "GET"
url = "https://api.example.com/health"
timeout = 10
headers = [
  { "Authorization" = "Bearer API_KEY" }
]

# Database Monitor
[[monitors]]
enabled = true
name = "Production Database"
interval = 60
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://uptime.example.com/api/push/db-token?latency={latency}"

[monitors.postgresql]
url = "postgresql://monitor:password@db.example.com:5432/production"
timeout = 5
use_tls = true

# Redis Cache Monitor
[[monitors]]
enabled = true
name = "Redis Cache"
interval = 15
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://uptime.example.com/api/push/redis-token?latency={latency}"

[monitors.redis]
url = "redis://:password@redis.example.com:6379/0"
timeout = 3
```

## WebSocket Mode Configuration

When using WebSocket mode, configuration is received from [UptimeMonitor-Server](https://github.com/Rabbit-Company/UptimeMonitor-Server). Set these environment variables:

```bash
export PULSE_SERVER_URL=http://localhost:3000
export PULSE_TOKEN=your_pulsemonitor_token
```

Or create a `.env` file:

```env
PULSE_SERVER_URL=http://localhost:3000
PULSE_TOKEN=your_pulsemonitor_token
```

In WebSocket mode:

- The `heartbeat` section is optional (server provides push endpoint)
- Configuration updates are received automatically
- No restart required for changes

## Configuration Priority

1. **Environment variables** (`PULSE_SERVER_URL` + `PULSE_TOKEN`) → WebSocket mode
2. **Config file** (`--config ./config.toml`) → File mode
3. **Default path** (`config.toml` in current directory) → File mode

## Validation

PulseMonitor validates configuration on startup. Common issues:

| Error                                      | Cause                            | Solution                                        |
| ------------------------------------------ | -------------------------------- | ----------------------------------------------- |
| "No configuration found"                   | Missing env vars and config file | Set environment variables or create config.toml |
| "Monitor does not contain X configuration" | Missing service section          | Add the appropriate service block               |
| "Unsupported HTTP method"                  | Invalid method                   | Use GET, POST, or HEAD                          |
| "connection timed out"                     | Network/firewall issue           | Check connectivity and timeout values           |
