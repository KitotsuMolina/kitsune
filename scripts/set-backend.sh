#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
CFG="${KITSUNE_CFG:-./config/base.conf}"
BACKEND="${1:-}"
if [[ "$BACKEND" != "cpu" && "$BACKEND" != "gpu" ]]; then
  echo "Uso: ./scripts/set-backend.sh <cpu|gpu>"
  exit 1
fi

sed -i "s/^backend=.*/backend=${BACKEND}/" $CFG
echo "[OK] backend=${BACKEND}"
echo "Reinicia para aplicar: ./scripts/stop.sh && ./scripts/start.sh"
