#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/vpnnode-target}"
export CARGO_TERM_COLOR=never
export VPNNODE_BASELINE_MODE=remote
export RUST_LOG="${RUST_LOG:-vpnnode=info,vpnnode::roles::client=debug,vpnnode::roles::relay=debug,vpnnode::roles::exit=debug}"

: "${VPNNODE_BASELINE_REMOTE_EXIT_ADDR:?set VPNNODE_BASELINE_REMOTE_EXIT_ADDR (for example 31.192.232.26:30000)}"

route_length="${VPNNODE_BASELINE_REMOTE_ROUTE_LENGTH:-3}"
if [[ "${route_length}" -ge 2 ]]; then
  : "${VPNNODE_BASELINE_REMOTE_RELAY1_ADDR:?set VPNNODE_BASELINE_REMOTE_RELAY1_ADDR (for example 45.197.133.115:30001)}"
fi
if [[ "${route_length}" -ge 3 ]]; then
  : "${VPNNODE_BASELINE_REMOTE_RELAY2_ADDR:?set VPNNODE_BASELINE_REMOTE_RELAY2_ADDR (for example 185.144.28.95:30002)}"
fi

export VPNNODE_BASELINE_REMOTE_LOCAL_LISTEN="${VPNNODE_BASELINE_REMOTE_LOCAL_LISTEN:-127.0.0.1:19080}"
export VPNNODE_BASELINE_REMOTE_ROUTE_CACHE_PATH="${VPNNODE_BASELINE_REMOTE_ROUTE_CACHE_PATH:-route_cache_remote_baseline.json}"
export VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS="${VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS:-200}"
export VPNNODE_BASELINE_REMOTE_HTTP_HOST="${VPNNODE_BASELINE_REMOTE_HTTP_HOST:-example}"

cd "$(dirname "$0")/.."
rm -f "${VPNNODE_BASELINE_REMOTE_ROUTE_CACHE_PATH}"

echo "=== remote baseline smoke ==="
echo "mode=${VPNNODE_BASELINE_MODE}"
echo "local_listen=${VPNNODE_BASELINE_REMOTE_LOCAL_LISTEN}"
echo "route_length=${VPNNODE_BASELINE_REMOTE_ROUTE_LENGTH:-3}"
echo "relay1=${VPNNODE_BASELINE_REMOTE_RELAY1_ADDR:-<unused>}"
echo "relay2=${VPNNODE_BASELINE_REMOTE_RELAY2_ADDR:-<unused>}"
echo "exit=${VPNNODE_BASELINE_REMOTE_EXIT_ADDR}"
echo "expected_status=${VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS}"
echo "http_host=${VPNNODE_BASELINE_REMOTE_HTTP_HOST}"

cargo test --test remote_baseline -- --test-threads=1 --nocapture
