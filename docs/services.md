# Service Monitors

PulseMonitor supports 13 different service types. Each monitor checks a service's availability and sends a heartbeat on success.

## Overview

| Service                                 | Protocol         | Default Timeout |
| --------------------------------------- | ---------------- | --------------- |
| [HTTP](#http)                           | HTTP/HTTPS       | 10s             |
| [WebSocket](#websocket)                 | WS/WSS           | 3s              |
| [TCP](#tcp)                             | TCP              | 5s              |
| [UDP](#udp)                             | UDP              | 3s              |
| [ICMP](#icmp)                           | ICMP (ping)      | 3s              |
| [SMTP](#smtp)                           | SMTP/SMTPS       | -               |
| [IMAP](#imap)                           | IMAP/IMAPS       | -               |
| [MySQL](#mysql)                         | MySQL            | 3s              |
| [MSSQL](#mssql)                         | TDS              | 3s              |
| [PostgreSQL](#postgresql)               | PostgreSQL       | 3s              |
| [Redis](#redis)                         | RESP             | 3s              |
| [Minecraft Java](#minecraft-java)       | MC Java Protocol | 3s              |
| [Minecraft Bedrock](#minecraft-bedrock) | MC Bedrock (UDP) | 3s              |

## HTTP

Monitor HTTP/HTTPS endpoints with configurable methods and headers.

### Configuration

```toml
[monitors.http]
method = "GET"                              # Required: GET, POST, HEAD
url = "https://api.example.com/health"      # Required: Full URL
timeout = 10                                # Optional: Seconds (default: 10)
headers = [                                 # Optional: Custom headers
  { "Authorization" = "Bearer TOKEN" },
  { "User-Agent" = "CustomAgent/1.0" }
]
```

### Options

| Option    | Type    | Default | Description                   |
| --------- | ------- | ------- | ----------------------------- |
| `method`  | string  | -       | HTTP method (GET, POST, HEAD) |
| `url`     | string  | -       | Full URL including protocol   |
| `timeout` | integer | 10      | Request timeout in seconds    |
| `headers` | array   | -       | Custom request headers        |

### Success Criteria

- HTTP response status code is 2xx (200-299)

### Examples

**Basic health check:**

```toml
[monitors.http]
method = "GET"
url = "https://api.example.com/health"
```

**Authenticated endpoint:**

```toml
[monitors.http]
method = "POST"
url = "https://api.example.com/v1/ping"
timeout = 15
headers = [
  { "Authorization" = "Bearer eyJhbGc..." },
  { "Content-Type" = "application/json" }
]
```

---

## WebSocket

Monitor WebSocket endpoints by sending a ping and waiting for pong.

### Configuration

```toml
[monitors.ws]
url = "wss://realtime.example.com/socket"   # Required: ws:// or wss://
timeout = 3                                  # Optional: Seconds (default: 3)
```

### Options

| Option    | Type    | Default | Description                      |
| --------- | ------- | ------- | -------------------------------- |
| `url`     | string  | -       | WebSocket URL (ws:// or wss://)  |
| `timeout` | integer | 3       | Pong response timeout in seconds |

### Success Criteria

- Connection established
- Pong response received matching sent ping payload

### Examples

**Secure WebSocket:**

```toml
[monitors.ws]
url = "wss://api.example.com/ws"
timeout = 5
```

**Local development:**

```toml
[monitors.ws]
url = "ws://localhost:8080/socket"
timeout = 2
```

---

## TCP

Monitor TCP port availability by establishing a connection.

### Configuration

```toml
[monitors.tcp]
host = "db.example.com"    # Required: Hostname or IP
port = 5432                # Required: Port number
timeout = 5                # Optional: Seconds (default: 5)
```

### Options

| Option    | Type    | Default | Description                   |
| --------- | ------- | ------- | ----------------------------- |
| `host`    | string  | -       | Target hostname or IP address |
| `port`    | integer | -       | Target port (1-65535)         |
| `timeout` | integer | 5       | Connection timeout in seconds |

### Success Criteria

- TCP connection successfully established

### Examples

**Database port check:**

```toml
[monitors.tcp]
host = "postgres.example.com"
port = 5432
timeout = 3
```

**SSH availability:**

```toml
[monitors.tcp]
host = "server.example.com"
port = 22
timeout = 5
```

---

## UDP

Monitor UDP services by sending a packet and optionally waiting for response.

### Configuration

```toml
[monitors.udp]
host = "dns.example.com"     # Required: Hostname or IP
port = 53                    # Required: Port number
timeout = 3                  # Optional: Seconds (default: 3)
payload = "ping"             # Optional: Data to send (default: "ping")
expect_response = false      # Optional: Wait for response (default: false)
```

### Options

| Option            | Type    | Default | Description                   |
| ----------------- | ------- | ------- | ----------------------------- |
| `host`            | string  | -       | Target hostname or IP address |
| `port`            | integer | -       | Target port (1-65535)         |
| `timeout`         | integer | 3       | Response timeout in seconds   |
| `payload`         | string  | "ping"  | Data to send                  |
| `expect_response` | boolean | false   | Whether to wait for response  |

### Success Criteria

- Packet sent successfully
- If `expect_response = true`: Response received within timeout

### Examples

**Fire-and-forget (logging service):**

```toml
[monitors.udp]
host = "syslog.example.com"
port = 514
payload = "<14>test"
expect_response = false
```

**Response expected:**

```toml
[monitors.udp]
host = "game-server.example.com"
port = 27015
payload = "\xFF\xFF\xFF\xFFTSource Engine Query"
expect_response = true
timeout = 2
```

---

## ICMP

Monitor hosts via ICMP ping. Returns actual round-trip latency.

### Configuration

```toml
[monitors.icmp]
host = "8.8.8.8"    # Required: Hostname or IP
timeout = 2         # Optional: Seconds (default: 3)
```

### Options

| Option    | Type    | Default | Description                   |
| --------- | ------- | ------- | ----------------------------- |
| `host`    | string  | -       | Target hostname or IP address |
| `timeout` | integer | 3       | Ping timeout in seconds       |

### Success Criteria

- Ping response received
- Latency extracted from ping output

### Requirements

ICMP requires elevated privileges:

**Linux:**

```bash
# Option 1: Run as root
sudo pulsemonitor --config config.toml

# Option 2: Add capability (recommended)
sudo setcap cap_net_raw+ep /usr/local/bin/pulsemonitor
```

**Docker:**

```yaml
services:
  pulsemonitor:
    image: rabbitcompany/pulsemonitor:3
    cap_add:
      - NET_RAW
```

### Examples

**Monitor gateway:**

```toml
[monitors.icmp]
host = "192.168.1.1"
timeout = 1
```

**Monitor external host:**

```toml
[monitors.icmp]
host = "google.com"
timeout = 3
```

---

## SMTP

Monitor SMTP servers by testing the connection.

### Configuration

```toml
[monitors.smtp]
url = "smtps://user:pass@mail.example.com:465"    # Required: SMTP URL
```

### URL Formats

| Format                                   | Description                 |
| ---------------------------------------- | --------------------------- |
| `smtps://user:pass@host:465`             | SMTP over TLS (recommended) |
| `smtp://user:pass@host:587?tls=required` | SMTP with STARTTLS          |
| `smtp://user:pass@host:25`               | Unencrypted SMTP            |

### Success Criteria

- Connection established
- Authentication successful (if credentials provided)

### Examples

**TLS connection (port 465):**

```toml
[monitors.smtp]
url = "smtps://monitor@example.com:password123@smtp.example.com:465"
```

**STARTTLS (port 587):**

```toml
[monitors.smtp]
url = "smtp://monitor@example.com:password123@smtp.example.com:587?tls=required"
```

---

## IMAP

Monitor IMAP servers by authenticating and logging out.

### Configuration

```toml
[monitors.imap]
server = "imap.example.com"      # Required: IMAP server hostname
port = 993                       # Required: Port (993 for IMAPS)
username = "monitor@example.com" # Required: Login username
password = "password123"         # Required: Login password
```

### Options

| Option     | Type    | Default | Description                              |
| ---------- | ------- | ------- | ---------------------------------------- |
| `server`   | string  | -       | IMAP server hostname                     |
| `port`     | integer | -       | Port number (993 for TLS, 143 for plain) |
| `username` | string  | -       | Authentication username                  |
| `password` | string  | -       | Authentication password                  |

### Success Criteria

- TLS connection established (port 993)
- Login successful
- Logout completed

### Examples

**Standard IMAPS:**

```toml
[monitors.imap]
server = "imap.gmail.com"
port = 993
username = "monitor@gmail.com"
password = "app_password"
```

---

## MySQL

Monitor MySQL/MariaDB by executing a test query.

### Configuration

```toml
[monitors.mysql]
url = "mysql://user:pass@localhost:3306/database"   # Required
timeout = 3                                          # Optional: Seconds (default: 3)
```

### URL Format

```
mysql://username:password@hostname:port/database?options
```

Common options:

- `require_ssl=true` - Require SSL connection
- `ssl_mode=REQUIRED` - SSL mode setting

### Success Criteria

- Connection established
- `SELECT 1` query succeeds

### Examples

**Local MySQL:**

```toml
[monitors.mysql]
url = "mysql://monitor:password@localhost:3306/mydb"
timeout = 3
```

**Remote with SSL:**

```toml
[monitors.mysql]
url = "mysql://monitor:password@db.example.com:3306/production?require_ssl=true"
timeout = 5
```

---

## MSSQL

Monitor Microsoft SQL Server by executing a test query.

### Configuration

```toml
[monitors.mssql]
url = "jdbc:sqlserver://host:1433;databaseName=db;user=sa;password=pass"   # Required
timeout = 3                                                                  # Optional
```

### URL Format (JDBC style)

```
jdbc:sqlserver://hostname:port;databaseName=database;user=username;password=password;options
```

Common options:

- `encrypt=true` - Enable encryption
- `TrustServerCertificate=true` - Trust self-signed certificates

### Success Criteria

- Connection established
- `SELECT 1` query succeeds

### Examples

**Standard connection:**

```toml
[monitors.mssql]
url = "jdbc:sqlserver://sql.example.com:1433;databaseName=master;user=monitor;password=MonitorPass123"
timeout = 5
```

**With encryption:**

```toml
[monitors.mssql]
url = "jdbc:sqlserver://sql.example.com:1433;databaseName=production;encrypt=true;user=monitor;password=Pass123;TrustServerCertificate=true"
timeout = 5
```

---

## PostgreSQL

Monitor PostgreSQL by executing a test query.

### Configuration

```toml
[monitors.postgresql]
url = "postgresql://user:pass@localhost:5432/database"   # Required
timeout = 3                                               # Optional: Seconds (default: 3)
use_tls = false                                          # Optional: Enable TLS (default: false)
```

### URL Format

```
postgresql://username:password@hostname:port/database
```

### Success Criteria

- Connection established
- `SELECT 1` query succeeds

### Examples

**Local PostgreSQL:**

```toml
[monitors.postgresql]
url = "postgresql://monitor:password@localhost:5432/mydb"
timeout = 3
```

**Remote with TLS:**

```toml
[monitors.postgresql]
url = "postgresql://monitor:password@db.example.com:5432/production"
timeout = 5
use_tls = true
```

---

## Redis

Monitor Redis by executing a PING command.

### Configuration

```toml
[monitors.redis]
url = "redis://user:pass@localhost:6379/0"   # Required
timeout = 3                                   # Optional: Seconds (default: 3)
```

### URL Format

```
redis://[username:password@]hostname:port[/database]
```

For Redis 6+ ACL authentication:

```
redis://username:password@hostname:port/0
```

For older Redis (password only):

```
redis://:password@hostname:port/0
```

### Success Criteria

- Connection established
- `PING` command returns `PONG`

### Examples

**Local Redis:**

```toml
[monitors.redis]
url = "redis://localhost:6379/0"
timeout = 2
```

**Authenticated Redis:**

```toml
[monitors.redis]
url = "redis://default:mypassword@redis.example.com:6379/0"
timeout = 3
```

**Redis Cluster node:**

```toml
[monitors.redis]
url = "redis://:cluster_password@redis-node-1.example.com:6379"
timeout = 3
```

---

## Minecraft Java

Monitor Minecraft Java Edition servers using the Server List Ping protocol. Returns server latency and current player count.

### Configuration

```toml
[monitors.minecraft-java]
host = "mc.example.com"    # Required: Server hostname or IP
port = 25565               # Optional: Server port (default: 25565)
timeout = 3                # Optional: Seconds (default: 3)
```

### Options

| Option    | Type    | Default | Description                   |
| --------- | ------- | ------- | ----------------------------- |
| `host`    | string  | -       | Server hostname or IP address |
| `port`    | integer | 25565   | Server port                   |
| `timeout` | integer | 3       | Ping timeout in seconds       |

### Success Criteria

- Server List Ping response received
- Server info (player count) returned

### Custom Metrics

Minecraft Java monitors populate the following custom metrics, available as template placeholders in heartbeat URLs and headers:

| Placeholder     | Metric    | Description                 |
| --------------- | --------- | --------------------------- |
| `{custom1}`     | `custom1` | Current online player count |
| `{playerCount}` | `custom1` | Alias for `{custom1}`       |

### Examples

**Standard server:**

```toml
[[monitors]]
enabled = true
name = "Minecraft Server"
interval = 30

[monitors.heartbeat]
method = "GET"
url = "https://uptime.example.com/api/push/TOKEN?latency={latency}&players={playerCount}"

[monitors.minecraft-java]
host = "mc.example.com"
```

**Custom port with timeout:**

```toml
[[monitors]]
enabled = true
name = "MC Java (Custom Port)"
interval = 60

[monitors.heartbeat]
method = "GET"
url = "https://uptime.example.com/api/push/TOKEN?latency={latency}"

[monitors.minecraft-java]
host = "192.168.1.100"
port = 25566
timeout = 10
```

---

## Minecraft Bedrock

Monitor Minecraft Bedrock Edition servers using the Bedrock ping protocol (UDP). Returns server latency and current player count.

### Configuration

```toml
[monitors.minecraft-bedrock]
host = "bedrock.example.com"   # Required: Server hostname or IP
port = 19132                   # Optional: Server port (default: 19132)
timeout = 3                    # Optional: Seconds (default: 3)
```

### Options

| Option    | Type    | Default | Description                   |
| --------- | ------- | ------- | ----------------------------- |
| `host`    | string  | -       | Server hostname or IP address |
| `port`    | integer | 19132   | Server port                   |
| `timeout` | integer | 3       | Ping timeout in seconds       |

### Success Criteria

- Bedrock ping response received
- Server info (player count) returned

### Custom Metrics

Minecraft Bedrock monitors populate the following custom metrics, available as template placeholders in heartbeat URLs and headers:

| Placeholder     | Metric    | Description                 |
| --------------- | --------- | --------------------------- |
| `{custom1}`     | `custom1` | Current online player count |
| `{playerCount}` | `custom1` | Alias for `{custom1}`       |

### Examples

**Standard Bedrock server:**

```toml
[[monitors]]
enabled = true
name = "Bedrock Server"
interval = 30

[monitors.heartbeat]
method = "GET"
url = "https://uptime.example.com/api/push/TOKEN?latency={latency}&players={playerCount}"

[monitors.minecraft-bedrock]
host = "bedrock.example.com"
```

**Custom port:**

```toml
[[monitors]]
enabled = true
name = "MC Bedrock (Custom Port)"
interval = 60

[monitors.heartbeat]
method = "GET"
url = "https://uptime.example.com/api/push/TOKEN?latency={latency}"

[monitors.minecraft-bedrock]
host = "192.168.1.100"
port = 19133
timeout = 10
```
