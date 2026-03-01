# PulseMonitor

A high-performance Rust monitoring agent that sends heartbeat pulses to uptime monitoring services. Supports HTTP, WebSocket, TCP, UDP, ICMP, SMTP, IMAP, MySQL, MSSQL, PostgreSQL, Redis, SNMP, Minecraft Java and Minecraft Bedrock monitoring.

## Features

| Feature                | Description                                                                                                 |
| ---------------------- | ----------------------------------------------------------------------------------------------------------- |
| **Multi-Protocol**     | Monitor HTTP, WS, TCP, UDP, ICMP, SMTP, IMAP, MySQL, MSSQL, PostgreSQL, Redis, SNMP, Minecraft Java/Bedrock |
| **Dual Mode**          | File-based config or centralized WebSocket management                                                       |
| **Reliable Delivery**  | Pulse retry queue with per-pulse acknowledgment ensures no data loss                                        |
| **Auto-Reconnect**     | Automatic reconnection with HTTP fallback when WebSocket is unavailable                                     |
| **Live Updates**       | Real-time configuration changes without restart (WebSocket mode)                                            |
| **Template Variables** | Dynamic placeholders for latency, timestamps, and custom metrics in heartbeat URLs                          |
| **Low Resource**       | Efficient Rust implementation with minimal overhead                                                         |

## Related Projects

| Project                                                                                | Description                               |
| -------------------------------------------------------------------------------------- | ----------------------------------------- |
| [UptimeMonitor-Server](https://github.com/Rabbit-Company/UptimeMonitor-Server)         | Backend server for centralized monitoring |
| [UptimeMonitor-StatusPage](https://github.com/Rabbit-Company/UptimeMonitor-StatusPage) | Public status page frontend               |

## Quick Start

### Docker

**WebSocket Mode** (centralized management):

```yaml
services:
  pulsemonitor:
    container_name: pulsemonitor
    image: rabbitcompany/pulsemonitor:3
    environment:
      - PULSE_SERVER_URL=http://your-server:3000
      - PULSE_TOKEN=your_token_here
    restart: unless-stopped
```

**File Mode** (standalone):

```yaml
services:
  pulsemonitor:
    container_name: pulsemonitor
    image: rabbitcompany/pulsemonitor:3
    volumes:
      - ./config.toml:/config.toml
    restart: unless-stopped
```

### Binary Installation

```bash
# Download and install
wget https://github.com/Rabbit-Company/PulseMonitor/releases/latest/download/pulsemonitor-$(uname -m)-gnu
sudo chmod 755 pulsemonitor-$(uname -m)-gnu
sudo mv pulsemonitor-$(uname -m)-gnu /usr/local/bin/pulsemonitor

# Run with config file
pulsemonitor --config ./config.toml

# Or use environment variables for WebSocket mode
export PULSE_SERVER_URL=http://localhost:3000
export PULSE_TOKEN=your_token_here
pulsemonitor
```

## Configuration Modes

### WebSocket Mode (Recommended)

Connect to [UptimeMonitor-Server](https://github.com/Rabbit-Company/UptimeMonitor-Server) for centralized management:

```bash
export PULSE_SERVER_URL=http://localhost:3000
export PULSE_TOKEN=your_token_here
```

Benefits:

- Centralized configuration management
- Real-time updates without restart
- Automatic reconnection on failure
- Reliable pulse delivery with retry queue and acknowledgment
- HTTP fallback when WebSocket is unavailable
- Multi-region deployment from single server

### File Mode

Create a `config.toml` for standalone operation. See [Configuration Guide](docs/configuration.md).

## Documentation

| Document                                     | Description                                                   |
| -------------------------------------------- | ------------------------------------------------------------- |
| [Configuration Guide](docs/configuration.md) | Complete `config.toml` reference with all monitor types       |
| [Service Monitors](docs/services.md)         | Detailed setup for each protocol (HTTP, TCP, databases, etc.) |
| [Deployment Guide](docs/deployment.md)       | Docker, systemd, and production deployment                    |
| [WebSocket Protocol](docs/websocket.md)      | Server communication and message formats                      |

## Minimal Configuration Example

```toml
[[monitors]]
enabled = true
name = "My API"
interval = 30

[monitors.heartbeat]
method = "GET"
url = "https://uptime.example.com/api/push/TOKEN?latency={latency}"

[monitors.http]
method = "GET"
url = "https://api.example.com/health"
timeout = 10
```

## Template Placeholders

Use these in heartbeat URLs and headers:

| Placeholder       | Description                     | Example                    |
| ----------------- | ------------------------------- | -------------------------- |
| `{latency}`       | Round-trip time in milliseconds | `123.456`                  |
| `{startTimeISO}`  | Check start time (ISO 8601)     | `2025-01-21T07:06:39.568Z` |
| `{endTimeISO}`    | Check end time (ISO 8601)       | `2025-01-21T07:06:40.000Z` |
| `{startTimeUnix}` | Check start time (Unix ms)      | `1753081599568`            |
| `{endTimeUnix}`   | Check end time (Unix ms)        | `1753081600000`            |

## Environment Variables

| Variable                      | Description                                              | Default | Required       |
| ----------------------------- | -------------------------------------------------------- | ------- | -------------- |
| `PULSE_LOG_LEVEL`             | Log verbosity: `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE` | `INFO`  | No             |
| `PULSE_SERVER_URL`            | UptimeMonitor-Server URL                                 | -       | WebSocket mode |
| `PULSE_TOKEN`                 | Authentication token                                     | -       | WebSocket mode |
| `PULSE_MAX_QUEUE_SIZE`        | Maximum number of pulses in retry queue                  | 10000   | No             |
| `PULSE_MAX_RETRIES`           | Maximum retry attempts per pulse before dropping         | 300     | No             |
| `PULSE_RETRY_DELAY_MS`        | Delay in milliseconds between retry attempts             | 10000   | No             |
| `PULSE_MAX_CONCURRENT_CHECKS` | Maximum number of simultaneous service checks            | 5000    | No             |

## License

[GPL-3.0](LICENSE)
