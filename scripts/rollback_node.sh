#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/hosts.env"

usage() {
  echo "Usage: $0 {exit|relay1|relay2}"
  exit 1
}

ROLE="${1:-}"
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

echo "==> Rolling back role=${ROLE} on ${USER}@${HOST} (unit=${UNIT_NAME})"

REMOTE_CMD=$(cat <<EOF
set -euo pipefail
if [[ ! -x "${VPNNODE_BIN_REMOTE}.prev" ]]; then
  echo "[remote] no previous binary found at ${VPNNODE_BIN_REMOTE}.prev; aborting rollback" >&2
  exit 1
fi

echo "[remote] stopping unit..."
systemctl stop "${UNIT_NAME}" || true

echo "[remote] restoring previous binary..."
cp "${VPNNODE_BIN_REMOTE}.prev" "${VPNNODE_BIN_REMOTE}"
chmod 0755 "${VPNNODE_BIN_REMOTE}"

echo "[remote] starting unit..."
systemctl start "${UNIT_NAME}"

echo "[remote] vpnnode health after rollback..."
"${VPNNODE_BIN_REMOTE}" health --config "${CONFIG_REMOTE}"

echo "[remote] recent journal (tail 50)..."
journalctl -u "${UNIT_NAME}" -n 50 --no-pager || true
EOF
)

set +e
ssh -q "${USER}@${HOST}" "${REMOTE_CMD}"
STATUS=$?
set -e

if [[ ${STATUS} -ne 0 ]]; then
  echo "!! rollback_node: ROLLBACK FAILED for role=${ROLE} on ${HOST} (exit=${STATUS})"
  exit ${STATUS}
fi

echo "==> Rollback SUCCESS for role=${ROLE} on ${HOST}"

