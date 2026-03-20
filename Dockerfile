FROM rust:latest as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:12-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/vpnnode /usr/local/bin/vpnnode
WORKDIR /data
ENTRYPOINT ["vpnnode"]

