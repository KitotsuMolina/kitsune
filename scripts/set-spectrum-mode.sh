#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
MODE="${1:-}"
if [[ "$MODE" != "single" && "$MODE" != "group" ]]; then
  echo "Uso: ./scripts/set-spectrum-mode.sh <single|group>"
  exit 1
fi

sed -i "s/^spectrum_mode=.*/spectrum_mode=${MODE}/" ./config/base.conf
echo "[OK] spectrum_mode=${MODE}"
echo "Reinicia para aplicar: ./scripts/stop.sh && ./scripts/start.sh"
