#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/vpnnode-target}"
export CARGO_TERM_COLOR=never
if [[ -n "${MSYS2_ENV_CONV_EXCL:-}" ]]; then
  export MSYS2_ENV_CONV_EXCL="${MSYS2_ENV_CONV_EXCL};VPNNODE_ROUTE_QUALITY_REQUEST_PATH;VPNNODE_ROUTE_QUALITY_RAW_PATH;VPNNODE_ROUTE_QUALITY_STAGE_TRACE_PATH;VPNNODE_ROUTE_QUALITY_REPORT_PATH;VPNNODE_BASELINE_REMOTE_RESET_CMD"
else
  export MSYS2_ENV_CONV_EXCL="VPNNODE_ROUTE_QUALITY_REQUEST_PATH;VPNNODE_ROUTE_QUALITY_RAW_PATH;VPNNODE_ROUTE_QUALITY_STAGE_TRACE_PATH;VPNNODE_ROUTE_QUALITY_REPORT_PATH;VPNNODE_BASELINE_REMOTE_RESET_CMD"
fi

export VPNNODE_BASELINE_MODE=remote
export RUST_LOG="${RUST_LOG:-vpnnode=info,vpnnode::roles::client=debug,vpnnode::roles::relay=debug,vpnnode::roles::exit=debug}"

: "${VPNNODE_BASELINE_REMOTE_EXIT_ADDR:?set VPNNODE_BASELINE_REMOTE_EXIT_ADDR}"
: "${VPNNODE_BASELINE_REMOTE_RELAY1_ADDR:?set VPNNODE_BASELINE_REMOTE_RELAY1_ADDR}"
: "${VPNNODE_BASELINE_REMOTE_RELAY2_ADDR:?set VPNNODE_BASELINE_REMOTE_RELAY2_ADDR}"

export VPNNODE_BASELINE_REMOTE_TARGET_SCHEME="${VPNNODE_BASELINE_REMOTE_TARGET_SCHEME:-http}"
export VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS="${VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS:-200}"
export VPNNODE_BASELINE_REMOTE_HTTP_HOST="${VPNNODE_BASELINE_REMOTE_HTTP_HOST:-example}"
export VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY=false
export VPNNODE_BASELINE_REMOTE_READY_TIMEOUT_SECS="${VPNNODE_BASELINE_REMOTE_READY_TIMEOUT_SECS:-90}"
export VPNNODE_BASELINE_REMOTE_PROBE_ATTEMPT_TIMEOUT_SECS="${VPNNODE_BASELINE_REMOTE_PROBE_ATTEMPT_TIMEOUT_SECS:-45}"
export VPNNODE_BASELINE_REMOTE_RESET_CMD="${VPNNODE_BASELINE_REMOTE_RESET_CMD:-}"

export VPNNODE_ROUTE_QUALITY_ROUTE_LENGTH="${VPNNODE_ROUTE_QUALITY_ROUTE_LENGTH:-3}"
export VPNNODE_ROUTE_QUALITY_LOCAL_LISTEN="${VPNNODE_ROUTE_QUALITY_LOCAL_LISTEN:-127.0.0.1:19380}"
export VPNNODE_ROUTE_QUALITY_ROUTE_CACHE_PATH="${VPNNODE_ROUTE_QUALITY_ROUTE_CACHE_PATH:-route_cache_route_quality.json}"
export VPNNODE_ROUTE_QUALITY_REQUEST_PATH="${VPNNODE_ROUTE_QUALITY_REQUEST_PATH:-/perf-262144.bin}"
export VPNNODE_ROUTE_QUALITY_RUNS="${VPNNODE_ROUTE_QUALITY_RUNS:-6}"
export VPNNODE_ROUTE_QUALITY_RAW_PATH="${VPNNODE_ROUTE_QUALITY_RAW_PATH:-docs/artifacts/route_quality_remote_2026-03-21.jsonl}"
export VPNNODE_ROUTE_QUALITY_STAGE_TRACE_PATH="${VPNNODE_ROUTE_QUALITY_STAGE_TRACE_PATH:-docs/artifacts/route_quality_remote_2026-03-21.client.jsonl}"
export VPNNODE_ROUTE_QUALITY_REPORT_PATH="${VPNNODE_ROUTE_QUALITY_REPORT_PATH:-docs/artifacts/route_quality_remote_2026-03-21.md}"
export VPNNODE_ROUTE_QUALITY_HTTP_HOST="${VPNNODE_ROUTE_QUALITY_HTTP_HOST:-route-quality}"
export VPNNODE_ROUTE_QUALITY_PROBE_HOST="${VPNNODE_ROUTE_QUALITY_PROBE_HOST:-route-quality-probe}"

cd "$(dirname "$0")/.."

rm -f \
  "${VPNNODE_ROUTE_QUALITY_RAW_PATH}" \
  "${VPNNODE_ROUTE_QUALITY_STAGE_TRACE_PATH}" \
  "${VPNNODE_ROUTE_QUALITY_REPORT_PATH}" \
  "${VPNNODE_ROUTE_QUALITY_ROUTE_CACHE_PATH}"

cleanup() {
  local status=$?
  trap - EXIT
  set +e
  echo "=== route-quality cleanup start $(date -Iseconds) ==="
  powershell.exe -NoProfile -Command "\
    \$port = [int](\"${VPNNODE_ROUTE_QUALITY_LOCAL_LISTEN}\".Split(':')[-1]); \
    \$listeners = Get-NetTCPConnection -LocalPort \$port -State Listen -ErrorAction SilentlyContinue; \
    if (\$listeners) { Write-Output \"route_quality_listener_present=\$port\"; exit 1 } \
    else { Write-Output \"route_quality_listener_present=none\"; exit 0 }"
  local cleanup_status=$?
  echo "=== route-quality cleanup end $(date -Iseconds) ==="
  if [[ $status -eq 0 && $cleanup_status -ne 0 ]]; then
    return "$cleanup_status"
  fi
  return "$status"
}
trap cleanup EXIT

echo "=== remote route-quality selection ==="
echo "relay1=${VPNNODE_BASELINE_REMOTE_RELAY1_ADDR}"
echo "relay2=${VPNNODE_BASELINE_REMOTE_RELAY2_ADDR}"
echo "exit=${VPNNODE_BASELINE_REMOTE_EXIT_ADDR}"
echo "route_length=${VPNNODE_ROUTE_QUALITY_ROUTE_LENGTH}"
echo "exact_route_only=${VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY}"
echo "request_path=${VPNNODE_ROUTE_QUALITY_REQUEST_PATH}"
echo "runs=${VPNNODE_ROUTE_QUALITY_RUNS}"
echo "local_listen=${VPNNODE_ROUTE_QUALITY_LOCAL_LISTEN}"
echo "raw_path=${VPNNODE_ROUTE_QUALITY_RAW_PATH}"
echo "stage_trace_path=${VPNNODE_ROUTE_QUALITY_STAGE_TRACE_PATH}"
echo "report_path=${VPNNODE_ROUTE_QUALITY_REPORT_PATH}"
if [[ -n "${VPNNODE_BASELINE_REMOTE_RESET_CMD}" ]]; then
  echo "reset_cmd=${VPNNODE_BASELINE_REMOTE_RESET_CMD}"
else
  echo "reset_cmd=<none>"
fi

cargo test --test remote_route_quality remote_route_quality_selection_uses_quality_feedback -- --test-threads=1 --nocapture
