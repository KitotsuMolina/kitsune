#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
CFG="${KITSUNE_CFG:-./config/base.conf}"
MODE="${1:-}"
if [[ "$MODE" != "bars" && "$MODE" != "ring" ]]; then
  echo "Uso: ./scripts/set-mode.sh <bars|ring>"
  exit 1
fi

sed -i "s/^mode=.*/mode=${MODE}/" $CFG

echo "[OK] mode=${MODE}"
