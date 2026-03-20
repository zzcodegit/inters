#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${SCRIPT_DIR%/scripts}"

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
    CONFIG_LOCAL="${REPO_ROOT}/${VPNNODE_EXIT_CONFIG_LOCAL}"
    CONFIG_REMOTE="${VPNNODE_EXIT_CONFIG_REMOTE}"
    UNIT_NAME="${VPNNODE_EXIT_UNIT}"
    UNIT_LOCAL="${REPO_ROOT}/${VPNNODE_EXIT_UNIT_LOCAL}"
    ;;
  relay1)
    HOST="${VPNNODE_RELAY1_HOST}"
    USER="${VPNNODE_RELAY1_USER}"
    CONFIG_LOCAL="${REPO_ROOT}/${VPNNODE_RELAY1_CONFIG_LOCAL}"
    CONFIG_REMOTE="${VPNNODE_RELAY1_CONFIG_REMOTE}"
    UNIT_NAME="${VPNNODE_RELAY1_UNIT}"
    UNIT_LOCAL="${REPO_ROOT}/${VPNNODE_RELAY1_UNIT_LOCAL}"
    ;;
  relay2)
    HOST="${VPNNODE_RELAY2_HOST}"
    USER="${VPNNODE_RELAY2_USER}"
    CONFIG_LOCAL="${REPO_ROOT}/${VPNNODE_RELAY2_CONFIG_LOCAL}"
    CONFIG_REMOTE="${VPNNODE_RELAY2_CONFIG_REMOTE}"
    UNIT_NAME="${VPNNODE_RELAY2_UNIT}"
    UNIT_LOCAL="${REPO_ROOT}/${VPNNODE_RELAY2_UNIT_LOCAL}"
    ;;
  *)
    usage
    ;;
esac

BIN_LOCAL="${REPO_ROOT}/target/release/vpnnode"

if [[ ! -x "${BIN_LOCAL}" ]]; then
  echo "error: vpnnode binary not found at ${BIN_LOCAL} (run 'cargo build --release' first)" >&2
  exit 1
fi

if [[ ! -f "${CONFIG_LOCAL}" ]]; then
  echo "error: config not found at ${CONFIG_LOCAL}" >&2
  exit 1
fi

if [[ ! -f "${UNIT_LOCAL}" ]]; then
  echo "error: unit file not found at ${UNIT_LOCAL}" >&2
  exit 1
fi

echo "==> Deploying role=${ROLE} to ${USER}@${HOST}"

TMP_BIN="/tmp/vpnnode-bin.$$"
TMP_CFG="/tmp/vpnnode-config-$(basename "${CONFIG_REMOTE}")-$$"
TMP_UNIT="/tmp/${UNIT_NAME}-$$"

echo "--> Copying binary and config to remote tmp"
scp -q "${BIN_LOCAL}" "${USER}@${HOST}:${TMP_BIN}"
scp -q "${CONFIG_LOCAL}" "${USER}@${HOST}:${TMP_CFG}"
scp -q "${UNIT_LOCAL}" "${USER}@${HOST}:${TMP_UNIT}"

REMOTE_CMD=$(cat <<EOF
set -euo pipefail
install -d -m 0755 /opt/vpnnode/bin
install -d -m 0755 /etc/vpnnode
install -d -m 0755 /var/lib/vpnnode

mv "${TMP_BIN}" "${VPNNODE_BIN_REMOTE}"
chmod 0755 "${VPNNODE_BIN_REMOTE}"

mv "${TMP_CFG}" "${CONFIG_REMOTE}"
chmod 0644 "${CONFIG_REMOTE}"

mv "${TMP_UNIT}" "${VPNNODE_UNIT_DIR_REMOTE}/${UNIT_NAME}"
chmod 0644 "${VPNNODE_UNIT_DIR_REMOTE}/${UNIT_NAME}"

systemctl daemon-reload
systemctl enable "${UNIT_NAME}" >/dev/null 2>&1 || true

echo "[remote] running vpnnode check..."
"${VPNNODE_BIN_REMOTE}" check --config "${CONFIG_REMOTE}"

echo "[remote] restarting systemd unit..."
systemctl restart "${UNIT_NAME}"

echo "[remote] running vpnnode health..."
"${VPNNODE_BIN_REMOTE}" health --config "${CONFIG_REMOTE}"
EOF
)

echo "--> Applying layout and restarting service on remote"
ssh -q "${USER}@${HOST}" "${REMOTE_CMD}"

echo "==> Deploy completed for role=${ROLE} (${USER}@${HOST}, unit=${UNIT_NAME})"

