# Deployment Guide

This guide covers deploying PulseMonitor in various environments.

## Docker Deployment

### WebSocket Mode (Recommended)

Connect to a centralized UptimeMonitor-Server:

```yaml
services:
  pulsemonitor:
    container_name: pulsemonitor
    image: rabbitcompany/pulsemonitor:3
    environment:
      - PULSE_SERVER_URL=http://uptime-server:3000
      - PULSE_TOKEN=your_pulsemonitor_token
    restart: unless-stopped
```

To tune the retry queue for high-scale deployments:

```yaml
services:
  pulsemonitor:
    container_name: pulsemonitor
    image: rabbitcompany/pulsemonitor:3
    environment:
      - PULSE_SERVER_URL=http://uptime-server:3000
      - PULSE_TOKEN=your_pulsemonitor_token
      - PULSE_MAX_QUEUE_SIZE=50000
      - PULSE_MAX_RETRIES=300
      - PULSE_RETRY_DELAY_MS=1000
    restart: unless-stopped
```

### File Mode

Use a local configuration file:

```yaml
services:
  pulsemonitor:
    container_name: pulsemonitor
    image: rabbitcompany/pulsemonitor:3
    volumes:
      - ./config.toml:/config.toml:ro
    restart: unless-stopped
```

## Binary Installation

### Download and Install

```bash
# Download latest release
wget https://github.com/Rabbit-Company/PulseMonitor/releases/latest/download/pulsemonitor-$(uname -m)-gnu

# Make executable
chmod 755 pulsemonitor-$(uname -m)-gnu

# Move to system path
sudo mv pulsemonitor-$(uname -m)-gnu /usr/local/bin/pulsemonitor

# Verify installation
pulsemonitor --version
```

### Run Manually

**File mode:**

```bash
pulsemonitor --config /path/to/config.toml
```

**WebSocket mode:**

```bash
export PULSE_SERVER_URL=http://localhost:3000
export PULSE_TOKEN=your_token
pulsemonitor
```

## Systemd Service

### WebSocket Mode

Create `/etc/systemd/system/pulsemonitor.service`:

```ini
[Unit]
Description=PulseMonitor Uptime Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=pulsemonitor
Group=pulsemonitor

# WebSocket mode configuration
Environment="PULSE_SERVER_URL=http://localhost:3000"
Environment="PULSE_TOKEN=your_token_here"

# Optional: tune retry queue for large deployments
# Environment="PULSE_MAX_QUEUE_SIZE=50000"
# Environment="PULSE_MAX_RETRIES=300"
# Environment="PULSE_RETRY_DELAY_MS=1000"

ExecStart=/usr/local/bin/pulsemonitor
Restart=always
RestartSec=5

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

### File Mode

Create `/etc/systemd/system/pulsemonitor.service`:

```ini
[Unit]
Description=PulseMonitor Uptime Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=pulsemonitor
Group=pulsemonitor

# File mode configuration
ExecStart=/usr/local/bin/pulsemonitor --config /etc/pulsemonitor/config.toml
Restart=always
RestartSec=5

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ReadOnlyPaths=/etc/pulsemonitor

[Install]
WantedBy=multi-user.target
```

### Enable and Start

```bash
# Create service user (optional but recommended)
sudo useradd -r -s /bin/false pulsemonitor

# Reload systemd
sudo systemctl daemon-reload

# Enable and start
sudo systemctl enable --now pulsemonitor

# Check status
sudo systemctl status pulsemonitor

# View logs
sudo journalctl -u pulsemonitor -f
```

### ICMP with Systemd

For ICMP monitoring, grant the capability:

```bash
sudo setcap cap_net_raw+ep /usr/local/bin/pulsemonitor
```

Or run as root (less secure):

```ini
[Service]
User=root
```

## Environment File

Create `/etc/pulsemonitor/.env` for cleaner configuration:

```env
PULSE_SERVER_URL=http://localhost:3000
PULSE_TOKEN=your_secure_token_here

# Optional: retry queue tuning
# PULSE_MAX_QUEUE_SIZE=50000
# PULSE_MAX_RETRIES=300
# PULSE_RETRY_DELAY_MS=1000
```

Reference in systemd:

```ini
[Service]
EnvironmentFile=/etc/pulsemonitor/.env
ExecStart=/usr/local/bin/pulsemonitor
```

Secure the file:

```bash
sudo chmod 600 /etc/pulsemonitor/.env
sudo chown pulsemonitor:pulsemonitor /etc/pulsemonitor/.env
```

## Upgrade Process

### Docker

```bash
# Pull latest image
sudo docker compose pull

# Stop the container
sudo docker compose down

# Start the container
sudo docker compose up -d
```

### Binary

```bash
# Download new version
wget https://github.com/Rabbit-Company/PulseMonitor/releases/latest/download/pulsemonitor-$(uname -m)-gnu

# Replace binary
sudo chmod 755 pulsemonitor-$(uname -m)-gnu
sudo mv pulsemonitor-$(uname -m)-gnu /usr/local/bin/pulsemonitor

# Restart service
sudo systemctl restart pulsemonitor
```

## Health Monitoring

### Check Service Status

```bash
# Systemd
sudo systemctl status pulsemonitor
sudo journalctl -u pulsemonitor --since "1 hour ago"

# Docker
docker logs pulsemonitor --tail 100
docker ps | grep pulsemonitor
```

### Enable Debug Logging

Set `debug = true` on individual monitors:

```toml
[[monitors]]
enabled = true
name = "Debug Monitor"
interval = 30
debug = true    # Enable verbose logging
```

## Scaling Considerations

When monitoring a large number of services (1,000+), consider tuning these parameters:

| Parameter              | Default | Recommendation           | Description                                      |
| ---------------------- | ------- | ------------------------ | ------------------------------------------------ |
| `PULSE_MAX_QUEUE_SIZE` | `10000` | `50000` for 1k+ monitors | Prevents pulse loss during extended outages      |
| `PULSE_MAX_RETRIES`    | `300`   | Keep default             | ~5 min retry window covers most transient issues |
| `PULSE_RETRY_DELAY_MS` | `1000`  | Keep default             | Balance between retry speed and server load      |

The retry queue holds one entry per unacknowledged pulse across all monitors. During a server outage, a PulseMonitor instance with 1,000 monitors at 10s intervals generates ~6,000 pulses per minute. With the default queue size of 10,000, this provides roughly 1.5 minutes of buffer. Increase `PULSE_MAX_QUEUE_SIZE` if longer outage tolerance is needed.

## Network Requirements

### Outbound Connections

PulseMonitor needs outbound access to:

| Destination          | Port                | Purpose              |
| -------------------- | ------------------- | -------------------- |
| UptimeMonitor-Server | 3000 (configurable) | WebSocket connection |
| Monitored services   | Various             | Health checks        |
| Heartbeat endpoints  | 443/80              | Pulse notifications  |

### Firewall Rules (iptables)

```bash
# Allow outbound HTTPS for heartbeats
iptables -A OUTPUT -p tcp --dport 443 -j ACCEPT

# Allow outbound to UptimeMonitor-Server
iptables -A OUTPUT -p tcp --dport 3000 -j ACCEPT

# Allow ICMP for ping monitoring
iptables -A OUTPUT -p icmp -j ACCEPT
```

## Troubleshooting

### Common Issues

| Issue                         | Cause                            | Solution                                                |
| ----------------------------- | -------------------------------- | ------------------------------------------------------- |
| "No configuration found"      | Missing env vars and config file | Set PULSE_SERVER_URL/PULSE_TOKEN or provide config.toml |
| "WebSocket connection failed" | Network/server issue             | Check server URL, network connectivity                  |
| "Authentication failed"       | Invalid token                    | Verify PULSE_TOKEN matches server configuration         |
| "Permission denied" (ICMP)    | Missing capability               | Run `setcap cap_net_raw+ep` or run as root              |
| "Connection timed out"        | Firewall/network issue           | Check firewall rules, verify target is reachable        |
| "Pulse queue full"            | Prolonged outage or high volume  | Increase PULSE_MAX_QUEUE_SIZE                           |

### Test Configuration

Validate config file syntax:

```bash
# This will fail fast if config is invalid
pulsemonitor --config config.toml
# Watch for "Configuration loaded" or error messages
```
