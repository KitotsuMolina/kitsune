#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
MODE="${1:-}"
if [[ "$MODE" != "standard" && "$MODE" != "test" ]]; then
  echo "Uso: ./scripts/set-runtime-mode.sh <standard|test>"
  exit 1
fi

sed -i "s/^runtime_mode=.*/runtime_mode=${MODE}/" ./config/base.conf

echo "[OK] runtime_mode=${MODE}"
