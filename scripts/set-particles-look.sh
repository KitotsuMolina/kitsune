#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

LAYER="${1:-}"
COLOR="${2:-}"

if [[ "$LAYER" != "front" && "$LAYER" != "back" ]]; then
  echo "Uso: ./scripts/set-particles-look.sh <front|back> <#RRGGBB|spectrum>"
  exit 1
fi

if [[ "$COLOR" == "spectrum" ]]; then
  sed -i "s/^particles_color_mode=.*/particles_color_mode=spectrum/" ./config/base.conf
else
  if ! [[ "$COLOR" =~ ^#?[0-9A-Fa-f]{6}$ ]]; then
    echo "[x] color invalido. Usa #RRGGBB o spectrum"
    exit 1
  fi
  if [[ "$COLOR" != \#* ]]; then
    COLOR="#$COLOR"
  fi
  sed -i "s/^particles_color=.*/particles_color=${COLOR}/" ./config/base.conf
  sed -i "s/^particles_color_mode=.*/particles_color_mode=static/" ./config/base.conf
fi

sed -i "s/^particles_layer=.*/particles_layer=${LAYER}/" ./config/base.conf

echo "[OK] particles_layer=${LAYER} particles_color_mode=$(awk -F= '$1=="particles_color_mode"{print $2}' ./config/base.conf) particles_color=$(awk -F= '$1=="particles_color"{print $2}' ./config/base.conf)"
echo "Reinicia: ./scripts/stop.sh && ./scripts/start.sh"
