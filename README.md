# PulseMonitor

This Rust program serves as a simple monitoring tool for HTTP, WS, ICMP, TCP, UDP, SMTP, IMAP, MySQL, MSSQL, PostgreSQL and Redis. It retrieves configurations from a `config.toml` file and performs monitoring tasks accordingly.

Features:

- Monitor HTTP, WS, ICMP, TCP, UDP, SMTP, IMAP, MySQL, MSSQL, PostgreSQL and Redis
- Sends regular pulses to specified uptime monitors
- Easily configurable via `config.toml` file

## Configuration

Before running Pulse Monitor, make sure to create `config.toml` file and configure all monitors.

```toml
#
# START HTTP
#
[[monitors]]
enabled = true
name = "rabbit-company.com"
interval = 10
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://example.com/api/push/token?status=up&msg=OK&ping={latency}"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

[monitors.http]
method = "GET"
timeout = 10
url = "https://rabbit-company.com"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

#
# START WS
#
[[monitors]]
enabled = true
name = "WS"
interval = 10
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://example.com/api/push/token?status=up&msg=OK&ping={latency}"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

[monitors.ws]
url = "ws://example.com"
timeout = 3

#
# START TCP
#
[[monitors]]
enabled = true
name = "TCP"
interval = 10
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://example.com/api/push/token?status=up&msg=OK&ping={latency}"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

[monitors.tcp]
host = "127.0.0.1"
port = 8080
timeout = 3

#
# START UDP
#
[[monitors]]
enabled = true
name = "UDP"
interval = 10
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://example.com/api/push/token?status=up&msg=OK&ping={latency}"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

[monitors.udp]
host = "127.0.0.1"
port = 9000
timeout = 2
payload = "ping"
expect_response = true

#
# START ICMP
#
[[monitors]]
enabled = true
name = "ICMP"
interval = 10
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://example.com/api/push/token?status=up&msg=OK&ping={latency}"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

[monitors.icmp]
host = "8.8.8.8"
timeout = 2

#
# START SMTP
#
[[monitors]]
enabled = true
name = "SMTP"
interval = 10
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://example.com/api/push/token?status=up&msg=OK&ping={latency}"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

[monitors.smtp]
# SMTP over TLS
url = "smtps://user:pass@hostname:port"
# SMTP with STARTTLS
#url = "smtp://user:pass@hostname:port?tls=required"
# Unencrypted SMTP
#url = "smtp://user:pass@hostname:port"

#
# START IMAP
#
[[monitors]]
enabled = true
name = "IMAP"
interval = 10
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://example.com/api/push/token?status=up&msg=OK&ping={latency}"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

[monitors.imap]
server = ""
port = 993
username = ""
password = ""

#
# START MySQL
#
[[monitors]]
enabled = true
name = "MySQL"
interval = 10
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://example.com/api/push/token?status=up&msg=OK&ping={latency}"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

[monitors.mysql]
url = "mysql://username:password@localhost:3306/db_name?require_ssl=true"
timeout = 3

#
# START MSSQL
#
[[monitors]]
enabled = true
name = "MSSQL"
interval = 10
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://example.com/api/push/token?status=up&msg=OK&ping={latency}"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

[monitors.mssql]
url = "jdbc:sqlserver://localhost:1433;databaseName=master;encrypt=true;user=sa;password=<password>;TrustServerCertificate=true;"
timeout = 3

#
# START PostgreSQL
#
[[monitors]]
enabled = true
name = "PostgreSQL"
interval = 10
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://example.com/api/push/token?status=up&msg=OK&ping={latency}"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

[monitors.postgresql]
url = "postgresql://username:password@localhost:5432/db_name"
timeout = 3
use_tls = false

#
# START Redis
#
[[monitors]]
enabled = true
name = "Redis"
interval = 10
debug = false

[monitors.heartbeat]
method = "GET"
timeout = 10
url = "https://example.com/api/push/token?status=up&msg=OK&ping={latency}"
headers = [
	#{ "Authorization" = "Bearer YOUR_BEARER_TOKEN" },
	{ "User-Agent" = "Mozilla/5.0 (compatible; PulseMonitor/3.8.4; +https://github.com/Rabbit-Company/PulseMonitor)" },
	{ "X-Latency" = "{latency}" },
]

[monitors.redis]
url = "redis://username:password@localhost:6379/db_name"
```

## Docker Installation

Do not forget to create `config.toml` file in the same directory as your `docker-compose.yml` file.

```yml
services:
  pulsemonitor:
    container_name: pulsemonitor
    image: "rabbitcompany/pulsemonitor:3"
    volumes:
      - ./config.toml:/config.toml
    restart: unless-stopped
```

## Installation

```bash
# Download the binary
wget https://github.com/Rabbit-Company/PulseMonitor/releases/latest/download/pulsemonitor-$(uname -m)-gnu
# Set file permissions
sudo chmod 755 pulsemonitor-$(uname -m)-gnu
# Place the binary to `/usr/local/bin`
sudo mv pulsemonitor-$(uname -m)-gnu /usr/local/bin/pulsemonitor
# Start the monitor and don't forget to change the path to your config.toml file
pulsemonitor --config ./config.toml
```

## Daemonizing (using systemd)

Running Pulse Monitor in the background is a simple task, just make sure that it runs without errors before doing this. Place the contents below in a file called `pulsemonitor.service` in the `/etc/systemd/system/` directory.

```service
[Unit]
Description=Pulse Monitor
After=network.target

[Service]
Type=simple
User=root
ExecStart=pulsemonitor --config ./config.toml
TimeoutStartSec=0
TimeoutStopSec=2
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
```

Then, run the commands below to reload systemd and start Pulse Monitor.

```bash
systemctl enable --now pulsemonitor
```

## Upgrade

```bash
# Stop service
systemctl stop pulsemonitor

# Download Pulse Monitor
wget https://github.com/Rabbit-Company/PulseMonitor/releases/latest/download/pulsemonitor-$(uname -m)-gnu
sudo chmod 755 pulsemonitor-$(uname -m)-gnu
sudo mv pulsemonitor-$(uname -m)-gnu /usr/local/bin/pulsemonitor

# Start service
systemctl start pulsemonitor
```
