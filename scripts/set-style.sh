#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
TARGET="${1:-}"
STYLE="${2:-}"

if [[ "$TARGET" != "bars" && "$TARGET" != "ring" ]]; then
  echo "Uso: ./scripts/set-style.sh <bars|ring> <bars|bars_fill|waves|waves_fill|dots>"
  exit 1
fi

if [[ "$STYLE" != "bars" && "$STYLE" != "bars_fill" && "$STYLE" != "waves" && "$STYLE" != "waves_fill" && "$STYLE" != "dots" ]]; then
  echo "Uso: ./scripts/set-style.sh <bars|ring> <bars|bars_fill|waves|waves_fill|dots>"
  exit 1
fi

KEY="${TARGET}_style"
sed -i "s/^${KEY}=.*/${KEY}=${STYLE}/" ./config/base.conf

echo "[OK] ${KEY}=${STYLE}"
CURRENT_MODE="$(awk -F'=' '$1 ~ /^mode$/ {print $2}' ./config/base.conf | tr -d '[:space:]')"
if [[ "$CURRENT_MODE" != "$TARGET" ]]; then
  echo "[i] Nota: mode actual es '${CURRENT_MODE}'. Para verlo debes poner mode=${TARGET}."
  echo "    Usa: ./scripts/set-visual.sh ${TARGET} ${STYLE}"
fi
