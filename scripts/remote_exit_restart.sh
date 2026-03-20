#!/usr/bin/env bash
set -euo pipefail

killall vpnnode 2>/dev/null || true
rm -f /root/vpnnode-exit.log

nohup env RUST_LOG=debug /root/vpnnode/target/release/vpnnode run --config /root/vpnnode-exit.toml > /root/vpnnode-exit.log 2>&1 &
sleep 0.5
tail -n 120 /root/vpnnode-exit.log || true

