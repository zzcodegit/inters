#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="/tmp/vpnnode-target"
export CARGO_TERM_COLOR=never
export VPNNODE_BASELINE_MODE=local

cd "$(dirname "$0")/.."

for i in 1 2 3 4 5; do
  echo "=== baseline run $i/5 ==="
  cargo test --test baseline -- --test-threads=1
done
