#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
CFG="${KITSUNE_CFG:-./config/base.conf}"
VAL="${1:-}"
if [[ "$VAL" != "0" && "$VAL" != "1" ]]; then
  echo "Uso: ./scripts/set-rotate-profiles.sh <0|1>"
  exit 1
fi

sed -i "s/^rotate_profiles=.*/rotate_profiles=${VAL}/" $CFG

echo "[OK] rotate_profiles=${VAL}"
