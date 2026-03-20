#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/hosts.env"

usage() {
  echo "Usage: $0 {exit|relay1|relay2} [journal-lines]"
  exit 1
}

ROLE="${1:-}"
LINES="${2:-100}"
if [[ -z "${ROLE}" ]]; then
  usage
fi

case "${ROLE}" in
  exit)
    HOST="${VPNNODE_EXIT_HOST}"
    USER="${VPNNODE_EXIT_USER}"
    UNIT_NAME="${VPNNODE_EXIT_UNIT}"
    ;;
  relay1)
    HOST="${VPNNODE_RELAY1_HOST}"
    USER="${VPNNODE_RELAY1_USER}"
    UNIT_NAME="${VPNNODE_RELAY1_UNIT}"
    ;;
  relay2)
    HOST="${VPNNODE_RELAY2_HOST}"
    USER="${VPNNODE_RELAY2_USER}"
    UNIT_NAME="${VPNNODE_RELAY2_UNIT}"
    ;;
  *)
    usage
    ;;
esac

echo "==> Tailing last ${LINES} lines of journal for ${ROLE} (${UNIT_NAME}) on ${USER}@${HOST}"

ssh -q "${USER}@${HOST}" "journalctl -u ${UNIT_NAME} -n ${LINES} --no-pager"

