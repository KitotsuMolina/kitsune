#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
VAL="${1:-}"
if [[ "$VAL" != "0" && "$VAL" != "1" ]]; then
  echo "Uso: ./scripts/set-dynamic-color.sh <0|1>"
  exit 1
fi

sed -i "s/^dynamic_color=.*/dynamic_color=${VAL}/" ./config/base.conf

echo "[OK] dynamic_color=${VAL}"
