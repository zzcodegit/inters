#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  echo "Usage: $0 [drain_timeout_sec]"
  echo
  echo "Default rolling order: relay2 -> relay1 -> exit"
  exit 1
}

DRAIN_TIMEOUT="${1:-30}"

if [[ "${DRAIN_TIMEOUT}" =~ ^[0-9]+$ ]]; then
  :
else
  usage
fi

ROLLING_ORDER=("relay2" "relay1" "exit")

echo "==> Starting rolling update (order: relay2 -> relay1 -> exit, drain_timeout=${DRAIN_TIMEOUT}s)"

for ROLE in "${ROLLING_ORDER[@]}"; do
  echo
  echo "==== Updating ${ROLE} ===="
  if ! "${SCRIPT_DIR}/safe_update_node.sh" "${ROLE}" "${DRAIN_TIMEOUT}"; then
    echo "!! rolling_update: update FAILED on ${ROLE}"
    echo "   You may rollback this node with: scripts/rollback_node.sh ${ROLE}"
    exit 1
  fi
done

echo
echo "==> Rolling update COMPLETED successfully"

