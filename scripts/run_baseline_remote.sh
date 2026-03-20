#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/vpnnode-target}"
export CARGO_TERM_COLOR=never
export VPNNODE_BASELINE_MODE=remote
export RUST_LOG="${RUST_LOG:-vpnnode=info,vpnnode::roles::client=debug,vpnnode::roles::relay=debug,vpnnode::roles::exit=debug}"

: "${VPNNODE_BASELINE_REMOTE_EXIT_ADDR:?set VPNNODE_BASELINE_REMOTE_EXIT_ADDR (for example 31.192.232.26:30000)}"
: "${VPNNODE_BASELINE_REMOTE_RELAY1_ADDR:?set VPNNODE_BASELINE_REMOTE_RELAY1_ADDR (for example 45.197.133.115:30001)}"
: "${VPNNODE_BASELINE_REMOTE_RELAY2_ADDR:?set VPNNODE_BASELINE_REMOTE_RELAY2_ADDR (for example 185.144.28.95:30002)}"

export VPNNODE_BASELINE_REMOTE_LOCAL_LISTEN="${VPNNODE_BASELINE_REMOTE_LOCAL_LISTEN:-127.0.0.1:19080}"
export VPNNODE_BASELINE_REMOTE_ROUTE_CACHE_PATH="${VPNNODE_BASELINE_REMOTE_ROUTE_CACHE_PATH:-route_cache_remote_baseline.json}"
export VPNNODE_BASELINE_REMOTE_TARGET_SCHEME="${VPNNODE_BASELINE_REMOTE_TARGET_SCHEME:-http}"
export VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS="${VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS:-200}"
export VPNNODE_BASELINE_REMOTE_HTTP_HOST="${VPNNODE_BASELINE_REMOTE_HTTP_HOST:-example}"
export VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY="${VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY:-true}"
export VPNNODE_BASELINE_REMOTE_READY_TIMEOUT_SECS="${VPNNODE_BASELINE_REMOTE_READY_TIMEOUT_SECS:-90}"
export VPNNODE_BASELINE_REMOTE_PROBE_ATTEMPT_TIMEOUT_SECS="${VPNNODE_BASELINE_REMOTE_PROBE_ATTEMPT_TIMEOUT_SECS:-45}"
export VPNNODE_BASELINE_REMOTE_RESET_CMD="${VPNNODE_BASELINE_REMOTE_RESET_CMD:-}"

cd "$(dirname "$0")/.."
rm -f "${VPNNODE_BASELINE_REMOTE_ROUTE_CACHE_PATH}" route_cache_remote_1hop.json route_cache_remote_2hop.json route_cache_remote_3hop.json

echo "=== remote baseline matrix ==="
echo "mode=${VPNNODE_BASELINE_MODE}"
echo "relay1=${VPNNODE_BASELINE_REMOTE_RELAY1_ADDR:-<unused>}"
echo "relay2=${VPNNODE_BASELINE_REMOTE_RELAY2_ADDR:-<unused>}"
echo "exit=${VPNNODE_BASELINE_REMOTE_EXIT_ADDR}"
echo "target_scheme=${VPNNODE_BASELINE_REMOTE_TARGET_SCHEME}"
echo "expected_status=${VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS}"
echo "exact_route_only=${VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY}"
echo "ready_timeout_secs=${VPNNODE_BASELINE_REMOTE_READY_TIMEOUT_SECS}"
echo "probe_attempt_timeout_secs=${VPNNODE_BASELINE_REMOTE_PROBE_ATTEMPT_TIMEOUT_SECS}"
if [[ -n "${VPNNODE_BASELINE_REMOTE_RESET_CMD}" ]]; then
  echo "reset_cmd=${VPNNODE_BASELINE_REMOTE_RESET_CMD}"
else
  echo "reset_cmd=<none>"
fi
echo "scenarios=remote_http_1hop_returns_200,remote_http_2hop_returns_200,remote_http_3hop_returns_200"

scenarios=(
  remote_http_1hop_returns_200
  remote_http_2hop_returns_200
  remote_http_3hop_returns_200
)

for scenario in "${scenarios[@]}"; do
  echo "=== scenario ${scenario} start $(date -Iseconds) ==="
  if [[ -n "${VPNNODE_BASELINE_REMOTE_RESET_CMD}" ]]; then
    echo "=== scenario ${scenario} reset-start $(date -Iseconds) ==="
    eval "${VPNNODE_BASELINE_REMOTE_RESET_CMD}"
    echo "=== scenario ${scenario} reset-end $(date -Iseconds) ==="
  fi
  cargo test --test remote_baseline "${scenario}" -- --test-threads=1 --nocapture
  echo "=== scenario ${scenario} end $(date -Iseconds) ==="
done
