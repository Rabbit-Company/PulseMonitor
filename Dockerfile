# Stage 1: Build
FROM rust:1.86 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

# Stage 2: Runtime
FROM debian:12-slim

LABEL maintainer="Rabbit Company (info@rabbit-company.com)"

RUN apt-get update && apt-get install -y \
	libssl3 \
	libssl-dev \
	curl \
	&& rm -rf /var/lib/apt/lists/*

RUN useradd -m pulse
COPY --from=builder /app/target/release/pulsemonitor /usr/local/bin/pulsemonitor
RUN chown pulse:pulse /usr/local/bin/pulsemonitor

USER pulse

CMD ["/usr/local/bin/pulsemonitor", "--config", "/config.toml"]