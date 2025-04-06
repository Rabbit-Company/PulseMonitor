# PulseMonitor

This Rust program serves as a simple monitoring tool for MySQL, PostgreSQL and Redis databases. It retrieves configurations from a `config.toml` file and performs monitoring tasks accordingly.

Features:

- Monitor MySQL, PostgreSQL and Redis databases
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
execute_every = 10

[monitors.heartbeat]
method = "GET"
url = ""
#bearer_token = "" (optional)
#username = "" (optional)
#password = "" (optional)

[monitors.http]
method = "GET"
url = "https://rabbit-company.com"
#bearer_token = "" (optional)
#username = "" (optional)
#password = "" (optional)

#
# START MySQL
#
[[monitors]]
enabled = true
name = "MySQL"
execute_every = 10

[monitors.heartbeat]
method = "GET"
url = ""
#bearer_token = "" (optional)
#username = "" (optional)
#password = "" (optional)

[monitors.mysql]
url = "mysql://username:password@localhost:3306/db_name"

#
# START PostgreSQL
#
[[monitors]]
enabled = true
name = "PostgreSQL"
execute_every = 10

[monitors.heartbeat]
method = "GET"
url = ""
#bearer_token = "" (optional)
#username = "" (optional)
#password = "" (optional)

[monitors.postgresql]
url = "postgresql://username:password@localhost:5432/db_name"

#
# START Redis
#
[[monitors]]
enabled = true
name = "Redis"
execute_every = 10

[monitors.heartbeat]
method = "GET"
url = ""
#bearer_token = "" (optional)
#username = "" (optional)
#password = "" (optional)

[monitors.redis]
url = "redis://username:password@localhost:6379/db_name"
```

## Installation

```bash
# Download the binary
wget https://github.com/Rabbit-Company/PulseMonitor/releases/download/v1.0.0/pulsemonitor
# Set file permissions
sudo chmod 755 pulsemonitor
# Place the binary to `/usr/local/bin`
sudo mv pulsemonitor /usr/local/bin
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
wget https://github.com/Rabbit-Company/PulseMonitor/releases/download/v1.0.0/pulsemonitor
sudo chmod 755 pulsemonitor
sudo mv pulsemonitor /usr/local/bin

# Start service
systemctl start pulsemonitor
```
