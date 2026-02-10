#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
MODE="${1:-}"
if [[ "$MODE" != "bars" && "$MODE" != "ring" ]]; then
  echo "Uso: ./scripts/set-mode.sh <bars|ring>"
  exit 1
fi

sed -i "s/^mode=.*/mode=${MODE}/" ./config/base.conf

echo "[OK] mode=${MODE}"
