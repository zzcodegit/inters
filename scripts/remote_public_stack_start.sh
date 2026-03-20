#!/usr/bin/env bash
set -euo pipefail

# Stop any previous stack.
killall vpnnode 2>/dev/null || true
sleep 0.3

rm -f /root/vpnnode-target.log /root/vpnnode-exit.log

# Start remote target HTTP service (for smoke test).
nohup env RUST_LOG=info /root/vpnnode/target/release/vpnnode target --listen 127.0.0.1:8080 > /root/vpnnode-target.log 2>&1 &
sleep 0.3

# Start remote exit node.
nohup env RUST_LOG=debug /root/vpnnode/target/release/vpnnode run --config /root/vpnnode-exit.toml > /root/vpnnode-exit.log 2>&1 &
sleep 0.5

ss -ltnp | grep 8080 || true
ss -lunp | grep 30000 || true

echo "=== target log (tail) ==="
tail -n 20 /root/vpnnode-target.log || true
echo "=== exit log (tail) ==="
tail -n 40 /root/vpnnode-exit.log || true

