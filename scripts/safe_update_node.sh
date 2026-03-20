#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/hosts.env"

usage() {
  echo "Usage: $0 {exit|relay1|relay2} [drain_timeout_sec]"
  exit 1
}

ROLE="${1:-}"
DRAIN_TIMEOUT="${2:-30}" # seconds to wait for graceful stop
if [[ -z "${ROLE}" ]]; then
  usage
fi

case "${ROLE}" in
  exit)
    HOST="${VPNNODE_EXIT_HOST}"
    USER="${VPNNODE_EXIT_USER}"
    CONFIG_REMOTE="${VPNNODE_EXIT_CONFIG_REMOTE}"
    UNIT_NAME="${VPNNODE_EXIT_UNIT}"
    ;;
  relay1)
    HOST="${VPNNODE_RELAY1_HOST}"
    USER="${VPNNODE_RELAY1_USER}"
    CONFIG_REMOTE="${VPNNODE_RELAY1_CONFIG_REMOTE}"
    UNIT_NAME="${VPNNODE_RELAY1_UNIT}"
    ;;
  relay2)
    HOST="${VPNNODE_RELAY2_HOST}"
    USER="${VPNNODE_RELAY2_USER}"
    CONFIG_REMOTE="${VPNNODE_RELAY2_CONFIG_REMOTE}"
    UNIT_NAME="${VPNNODE_RELAY2_UNIT}"
    ;;
  *)
    usage
    ;;
esac

REPO_ROOT="${SCRIPT_DIR%/scripts}"
BIN_LOCAL="${REPO_ROOT}/target/release/vpnnode"

if [[ ! -x "${BIN_LOCAL}" ]]; then
  echo "error: vpnnode binary not found at ${BIN_LOCAL} (run 'cargo build --release' first)" >&2
  exit 1
fi

echo "==> Safe update for role=${ROLE} on ${USER}@${HOST} (unit=${UNIT_NAME}, drain_timeout=${DRAIN_TIMEOUT}s)"

TMP_BIN="/tmp/vpnnode-bin-update-$$"

echo "--> Uploading new binary to remote temp path"
scp -q "${BIN_LOCAL}" "${USER}@${HOST}:${TMP_BIN}"

REMOTE_CMD=$(cat <<EOF
set -euo pipefail
echo "[remote] step 1: vpnnode check (pre-update)..."
"${VPNNODE_BIN_REMOTE}" check --config "${CONFIG_REMOTE}"

echo "[remote] step 2: graceful stop via SIGTERM..."
systemctl kill -s SIGTERM "${UNIT_NAME}" || true

echo "[remote] waiting up to ${DRAIN_TIMEOUT}s for unit to stop..."
end=\$((\$(date +%s) + ${DRAIN_TIMEOUT}))
while systemctl is-active --quiet "${UNIT_NAME}"; do
  if [[ \$(date +%s) -ge \$end ]]; then
    echo "[remote] drain timeout reached, forcing stop..."
    systemctl stop "${UNIT_NAME}" || true
    break
  fi
  sleep 1
done

echo "[remote] step 3: backup existing binary (if present)..."
if [[ -x "${VPNNODE_BIN_REMOTE}" ]]; then
  cp "${VPNNODE_BIN_REMOTE}" "${VPNNODE_BIN_REMOTE}.prev"
fi

echo "[remote] step 4: replace binary..."
mv "${TMP_BIN}" "${VPNNODE_BIN_REMOTE}"
chmod 0755 "${VPNNODE_BIN_REMOTE}"

echo "[remote] step 5: start unit..."
systemctl start "${UNIT_NAME}"

echo "[remote] step 6: vpnnode health (post-update)..."
"${VPNNODE_BIN_REMOTE}" health --config "${CONFIG_REMOTE}"

echo "[remote] step 7: recent journal (tail 50)..."
journalctl -u "${UNIT_NAME}" -n 50 --no-pager || true
EOF
)

set +e
ssh -q "${USER}@${HOST}" "${REMOTE_CMD}"
STATUS=$?
set -e

if [[ ${STATUS} -ne 0 ]]; then
  echo "!! safe_update_node: update FAILED for role=${ROLE} on ${HOST} (exit=${STATUS})"
  echo "   You can attempt rollback with: scripts/rollback_node.sh ${ROLE}"
  exit ${STATUS}
fi

echo "==> Safe update SUCCESS for role=${ROLE} on ${HOST}"

