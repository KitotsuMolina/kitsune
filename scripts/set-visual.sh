#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
MODE="${1:-}"
STYLE="${2:-}"

if [[ "$MODE" != "bars" && "$MODE" != "ring" ]]; then
  echo "Uso: ./scripts/set-visual.sh <bars|ring> <bars|bars_fill|waves|waves_fill|dots>"
  exit 1
fi

if [[ "$STYLE" != "bars" && "$STYLE" != "bars_fill" && "$STYLE" != "waves" && "$STYLE" != "waves_fill" && "$STYLE" != "dots" ]]; then
  echo "Uso: ./scripts/set-visual.sh <bars|ring> <bars|bars_fill|waves|waves_fill|dots>"
  exit 1
fi

sed -i "s/^mode=.*/mode=${MODE}/" ./config/base.conf
sed -i "s/^${MODE}_style=.*/${MODE}_style=${STYLE}/" ./config/base.conf

echo "[OK] mode=${MODE} ${MODE}_style=${STYLE}"
echo "Reinicia para aplicar: ./scripts/stop.sh && ./scripts/start.sh"
