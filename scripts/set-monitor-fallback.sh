#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

ENABLED="${1:-}"
CHECK_SECONDS="${2:-}"
PREFER_FOCUSED="${3:-}"

if [[ "$ENABLED" != "0" && "$ENABLED" != "1" ]]; then
  echo "Uso: ./scripts/set-monitor-fallback.sh <enabled:0|1> [check_seconds>=1] [prefer_focused:0|1]"
  exit 1
fi

if [[ -n "$CHECK_SECONDS" ]]; then
  if ! [[ "$CHECK_SECONDS" =~ ^[0-9]+$ ]] || [[ "$CHECK_SECONDS" -lt 1 ]]; then
    echo "[x] check_seconds debe ser entero >= 1"
    exit 1
  fi
fi

if [[ -n "$PREFER_FOCUSED" && "$PREFER_FOCUSED" != "0" && "$PREFER_FOCUSED" != "1" ]]; then
  echo "[x] prefer_focused debe ser 0 o 1"
  exit 1
fi

sed -i "s/^monitor_fallback_enabled=.*/monitor_fallback_enabled=${ENABLED}/" ./config/base.conf

if [[ -n "$CHECK_SECONDS" ]]; then
  sed -i "s/^monitor_fallback_check_seconds=.*/monitor_fallback_check_seconds=${CHECK_SECONDS}/" ./config/base.conf
fi

if [[ -n "$PREFER_FOCUSED" ]]; then
  sed -i "s/^monitor_fallback_prefer_focused=.*/monitor_fallback_prefer_focused=${PREFER_FOCUSED}/" ./config/base.conf
fi

echo "[OK] monitor_fallback_enabled=$(awk -F= '$1=="monitor_fallback_enabled"{print $2}' ./config/base.conf) monitor_fallback_check_seconds=$(awk -F= '$1=="monitor_fallback_check_seconds"{print $2}' ./config/base.conf) monitor_fallback_prefer_focused=$(awk -F= '$1=="monitor_fallback_prefer_focused"{print $2}' ./config/base.conf)"
echo "Reinicia para aplicar: ./scripts/stop.sh && ./scripts/start.sh"
