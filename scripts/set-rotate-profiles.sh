#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
VAL="${1:-}"
if [[ "$VAL" != "0" && "$VAL" != "1" ]]; then
  echo "Uso: ./scripts/set-rotate-profiles.sh <0|1>"
  exit 1
fi

sed -i "s/^rotate_profiles=.*/rotate_profiles=${VAL}/" ./config/base.conf

echo "[OK] rotate_profiles=${VAL}"
