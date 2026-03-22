#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/vpnnode-target}"
export CARGO_TERM_COLOR=never
if [[ -n "${MSYS2_ENV_CONV_EXCL:-}" ]]; then
  export MSYS2_ENV_CONV_EXCL="${MSYS2_ENV_CONV_EXCL};VPNNODE_PERF_REQUEST_PATH;VPNNODE_PERF_SETUP_CMD;VPNNODE_PERF_STAGE_TRACE_SETUP_CMD;VPNNODE_PERF_STAGE_FETCH_CMD;VPNNODE_PERF_STAGE_CLEANUP_CMD;VPNNODE_PERF_STAGE_LOCAL_RAW_PATH;VPNNODE_STAGE_TRACE_PATH;VPNNODE_PERF_EXIT_STAGE_LOG_PATH;VPNNODE_PERF_FORWARD_LOG_PATH;VPNNODE_PERF_EXIT_STAGE_LOG_LOCAL_PATH;VPNNODE_PERF_DIRECT_STAGE_LOG_LOCAL_PATH;VPNNODE_PERF_STAGE_SUMMARY_PATH;VPNNODE_PERF_STAGE_JOINED_PATH;VPNNODE_BASELINE_REMOTE_RESET_CMD"
else
  export MSYS2_ENV_CONV_EXCL="VPNNODE_PERF_REQUEST_PATH;VPNNODE_PERF_SETUP_CMD;VPNNODE_PERF_STAGE_TRACE_SETUP_CMD;VPNNODE_PERF_STAGE_FETCH_CMD;VPNNODE_PERF_STAGE_CLEANUP_CMD;VPNNODE_PERF_STAGE_LOCAL_RAW_PATH;VPNNODE_STAGE_TRACE_PATH;VPNNODE_PERF_EXIT_STAGE_LOG_PATH;VPNNODE_PERF_FORWARD_LOG_PATH;VPNNODE_PERF_EXIT_STAGE_LOG_LOCAL_PATH;VPNNODE_PERF_DIRECT_STAGE_LOG_LOCAL_PATH;VPNNODE_PERF_STAGE_SUMMARY_PATH;VPNNODE_PERF_STAGE_JOINED_PATH;VPNNODE_BASELINE_REMOTE_RESET_CMD"
fi

export VPNNODE_BASELINE_MODE=remote
export RUST_LOG="${RUST_LOG:-vpnnode=info,vpnnode::roles::client=info,vpnnode::roles::exit=info}"

: "${VPNNODE_BASELINE_REMOTE_EXIT_ADDR:?set VPNNODE_BASELINE_REMOTE_EXIT_ADDR}"
: "${VPNNODE_BASELINE_REMOTE_RELAY1_ADDR:?set VPNNODE_BASELINE_REMOTE_RELAY1_ADDR}"
: "${VPNNODE_BASELINE_REMOTE_RELAY2_ADDR:?set VPNNODE_BASELINE_REMOTE_RELAY2_ADDR}"
: "${VPNNODE_PERF_DIRECT_ADDR:?set VPNNODE_PERF_DIRECT_ADDR}"

export VPNNODE_BASELINE_REMOTE_TARGET_SCHEME="${VPNNODE_BASELINE_REMOTE_TARGET_SCHEME:-http}"
export VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS="${VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS:-200}"
export VPNNODE_BASELINE_REMOTE_HTTP_HOST="${VPNNODE_BASELINE_REMOTE_HTTP_HOST:-example}"
export VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY="${VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY:-true}"
export VPNNODE_BASELINE_REMOTE_READY_TIMEOUT_SECS="${VPNNODE_BASELINE_REMOTE_READY_TIMEOUT_SECS:-90}"
export VPNNODE_BASELINE_REMOTE_PROBE_ATTEMPT_TIMEOUT_SECS="${VPNNODE_BASELINE_REMOTE_PROBE_ATTEMPT_TIMEOUT_SECS:-45}"
export VPNNODE_BASELINE_REMOTE_RESET_CMD="${VPNNODE_BASELINE_REMOTE_RESET_CMD:-}"

export VPNNODE_PERF_REQUEST_PATH="${VPNNODE_PERF_REQUEST_PATH:-/perf-262144.bin}"
export VPNNODE_PERF_RUNS="${VPNNODE_PERF_RUNS:-5}"
export VPNNODE_PERF_STAGE_LOCAL_RAW_PATH="${VPNNODE_PERF_STAGE_LOCAL_RAW_PATH:-docs/artifacts/remote_perf_stage_matrix_2026-03-21.local.jsonl}"
export VPNNODE_STAGE_TRACE_PATH="${VPNNODE_STAGE_TRACE_PATH:-docs/artifacts/remote_perf_stage_matrix_2026-03-21.client.jsonl}"
export VPNNODE_PERF_EXIT_STAGE_LOG_PATH="${VPNNODE_PERF_EXIT_STAGE_LOG_PATH:-/var/log/vpnnode/overlay-stage-exit.jsonl}"
export VPNNODE_PERF_FORWARD_LOG_PATH="${VPNNODE_PERF_FORWARD_LOG_PATH:-/var/log/vpnnode/perf-forward-stage.jsonl}"
export VPNNODE_PERF_EXIT_STAGE_LOG_LOCAL_PATH="${VPNNODE_PERF_EXIT_STAGE_LOG_LOCAL_PATH:-docs/artifacts/remote_perf_stage_matrix_2026-03-21.exit.jsonl}"
export VPNNODE_PERF_DIRECT_STAGE_LOG_LOCAL_PATH="${VPNNODE_PERF_DIRECT_STAGE_LOG_LOCAL_PATH:-docs/artifacts/remote_perf_stage_matrix_2026-03-21.direct.jsonl}"
export VPNNODE_PERF_STAGE_SUMMARY_PATH="${VPNNODE_PERF_STAGE_SUMMARY_PATH:-docs/artifacts/remote_perf_stage_matrix_2026-03-21.md}"
export VPNNODE_PERF_STAGE_JOINED_PATH="${VPNNODE_PERF_STAGE_JOINED_PATH:-docs/artifacts/remote_perf_stage_matrix_2026-03-21.joined.jsonl}"

export VPNNODE_PERF_SETUP_CMD="${VPNNODE_PERF_SETUP_CMD:-powershell.exe -ExecutionPolicy Bypass -File scripts/setup_perf_target_public.ps1}"
export VPNNODE_PERF_STAGE_TRACE_SETUP_CMD="${VPNNODE_PERF_STAGE_TRACE_SETUP_CMD:-powershell.exe -ExecutionPolicy Bypass -File scripts/setup_perf_stage_trace.ps1}"
export VPNNODE_PERF_STAGE_FETCH_CMD="${VPNNODE_PERF_STAGE_FETCH_CMD:-powershell.exe -ExecutionPolicy Bypass -File scripts/collect_perf_stage_artifacts.ps1}"
export VPNNODE_PERF_STAGE_CLEANUP_CMD="${VPNNODE_PERF_STAGE_CLEANUP_CMD:-powershell.exe -ExecutionPolicy Bypass -File scripts/cleanup_perf_stage_trace.ps1}"

cd "$(dirname "$0")/.."

rm -f "${VPNNODE_PERF_STAGE_LOCAL_RAW_PATH}" \
  "${VPNNODE_STAGE_TRACE_PATH}" \
  "${VPNNODE_PERF_EXIT_STAGE_LOG_LOCAL_PATH}" \
  "${VPNNODE_PERF_DIRECT_STAGE_LOG_LOCAL_PATH}" \
  "${VPNNODE_PERF_STAGE_JOINED_PATH}" \
  "${VPNNODE_PERF_STAGE_SUMMARY_PATH}" \
  route_cache_perf_stage_1hop.json route_cache_perf_stage_2hop.json route_cache_perf_stage_3hop.json

cleanup() {
  local status=$?
  trap - EXIT
  set +e
  echo "=== cleanup start $(date -Iseconds) ==="
  local cleanup_status=0
  if [[ -n "${VPNNODE_PERF_STAGE_CLEANUP_CMD}" ]]; then
    eval "${VPNNODE_PERF_STAGE_CLEANUP_CMD}"
    cleanup_status=$?
  fi
  echo "=== cleanup end $(date -Iseconds) ==="
  if [[ $status -eq 0 && $cleanup_status -ne 0 ]]; then
    return "$cleanup_status"
  fi
  return "$status"
}
trap cleanup EXIT

echo "=== remote perf stage matrix ==="
echo "direct=${VPNNODE_PERF_DIRECT_ADDR}"
echo "relay1=${VPNNODE_BASELINE_REMOTE_RELAY1_ADDR}"
echo "relay2=${VPNNODE_BASELINE_REMOTE_RELAY2_ADDR}"
echo "exit=${VPNNODE_BASELINE_REMOTE_EXIT_ADDR}"
echo "request_path=${VPNNODE_PERF_REQUEST_PATH}"
echo "runs=${VPNNODE_PERF_RUNS}"
echo "stage_local_raw=${VPNNODE_PERF_STAGE_LOCAL_RAW_PATH}"
echo "client_stage_path=${VPNNODE_STAGE_TRACE_PATH}"
echo "exit_stage_path=${VPNNODE_PERF_EXIT_STAGE_LOG_PATH}"
echo "direct_stage_path=${VPNNODE_PERF_FORWARD_LOG_PATH}"
echo "joined_stage_path=${VPNNODE_PERF_STAGE_JOINED_PATH}"

echo "=== target setup start $(date -Iseconds) ==="
eval "${VPNNODE_PERF_SETUP_CMD}"
echo "=== target setup end $(date -Iseconds) ==="

echo "=== exit stage trace setup start $(date -Iseconds) ==="
eval "${VPNNODE_PERF_STAGE_TRACE_SETUP_CMD}"
echo "=== exit stage trace setup end $(date -Iseconds) ==="

cargo test --test remote_perf_stages remote_perf_stage_matrix_collects_measurements -- --test-threads=1 --nocapture

echo "=== collect stage artifacts start $(date -Iseconds) ==="
eval "${VPNNODE_PERF_STAGE_FETCH_CMD}"
echo "=== collect stage artifacts end $(date -Iseconds) ==="

echo "=== summarize stage artifacts start $(date -Iseconds) ==="
python scripts/summarize_perf_stages.py
echo "=== summarize stage artifacts end $(date -Iseconds) ==="
