#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
CFG="${KITSUNE_CFG:-./config/base.conf}"

LAYER="${1:-}"
COLOR="${2:-}"

if [[ "$LAYER" != "front" && "$LAYER" != "back" ]]; then
  echo "Uso: ./scripts/set-particles-look.sh <front|back> <#RRGGBB|spectrum>"
  exit 1
fi

if [[ "$COLOR" == "spectrum" ]]; then
  sed -i "s/^particles_color_mode=.*/particles_color_mode=spectrum/" $CFG
else
  if ! [[ "$COLOR" =~ ^#?[0-9A-Fa-f]{6}$ ]]; then
    echo "[x] color invalido. Usa #RRGGBB o spectrum"
    exit 1
  fi
  if [[ "$COLOR" != \#* ]]; then
    COLOR="#$COLOR"
  fi
  sed -i "s/^particles_color=.*/particles_color=${COLOR}/" $CFG
  sed -i "s/^particles_color_mode=.*/particles_color_mode=static/" $CFG
fi

sed -i "s/^particles_layer=.*/particles_layer=${LAYER}/" $CFG

echo "[OK] particles_layer=${LAYER} particles_color_mode=$(awk -F= '$1=="particles_color_mode"{print $2}' $CFG) particles_color=$(awk -F= '$1=="particles_color"{print $2}' $CFG)"
echo "Reinicia: ./scripts/stop.sh && ./scripts/start.sh"
