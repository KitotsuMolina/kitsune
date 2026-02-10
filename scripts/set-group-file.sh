#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
FILE="${1:-}"
if [[ -z "$FILE" ]]; then
  echo "Uso: ./scripts/set-group-file.sh <ruta.group>"
  exit 1
fi

sed -i "s|^group_file=.*|group_file=${FILE}|" ./config/base.conf
echo "[OK] group_file=${FILE}"
echo "Reinicia para aplicar: ./scripts/stop.sh && ./scripts/start.sh"
