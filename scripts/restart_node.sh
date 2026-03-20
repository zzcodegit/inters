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
    UNIT_NAME="${VPNNODE_EXIT_UNIT}"
    CONFIG_REMOTE="${VPNNODE_EXIT_CONFIG_REMOTE}"
    ;;
  relay1)
    HOST="${VPNNODE_RELAY1_HOST}"
    USER="${VPNNODE_RELAY1_USER}"
    UNIT_NAME="${VPNNODE_RELAY1_UNIT}"
    CONFIG_REMOTE="${VPNNODE_RELAY1_CONFIG_REMOTE}"
    ;;
  relay2)
    HOST="${VPNNODE_RELAY2_HOST}"
    USER="${VPNNODE_RELAY2_USER}"
    UNIT_NAME="${VPNNODE_RELAY2_UNIT}"
    CONFIG_REMOTE="${VPNNODE_RELAY2_CONFIG_REMOTE}"
    ;;
  *)
    usage
    ;;
esac

echo "==> Restarting role=${ROLE} on ${USER}@${HOST} (unit=${UNIT_NAME})"

REMOTE_CMD=$(cat <<EOF
set -euo pipefail
echo "[remote] vpnnode check..."
"${VPNNODE_BIN_REMOTE}" check --config "${CONFIG_REMOTE}"
echo "[remote] systemctl restart ${UNIT_NAME}..."
systemctl restart "${UNIT_NAME}"
echo "[remote] vpnnode health..."
"${VPNNODE_BIN_REMOTE}" health --config "${CONFIG_REMOTE}"
EOF
)

ssh -q "${USER}@${HOST}" "${REMOTE_CMD}"

echo "==> Restart OK for role=${ROLE}"

