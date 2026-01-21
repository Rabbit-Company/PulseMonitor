# Stage 1: Build
FROM rust:1.92-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev pkgconfig build-base

WORKDIR /app
COPY . .
RUN cargo build --release

# Stage 2: Runtime
FROM alpine

LABEL maintainer="Rabbit Company (info@rabbit-company.com)"

RUN adduser -D -u 1000 pulse

COPY --from=builder /app/target/release/pulsemonitor /usr/local/bin/pulsemonitor
RUN chown pulse:pulse /usr/local/bin/pulsemonitor

USER pulse

CMD ["/usr/local/bin/pulsemonitor", "--config", "/config.toml"]
