#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
PRESET="${1:-}"

if [[ "$PRESET" != "clean" && "$PRESET" != "impact" ]]; then
  echo "Uso: ./scripts/set-waves-fill-preset.sh <clean|impact>"
  exit 1
fi

set_key() {
  local key="$1"
  local val="$2"
  sed -i "s/^${key}=.*/${key}=${val}/" ./config/base.conf
}

if [[ "$PRESET" == "clean" ]]; then
  set_key ring_wave_roundness 0.82
  set_key ring_fill_softness 0.25
  set_key ring_fill_overlap_px 1.80
  set_key ring_wave_thickness 2
else
  set_key ring_wave_roundness 0.72
  set_key ring_fill_softness 0.55
  set_key ring_fill_overlap_px 2.40
  set_key ring_wave_thickness 2
fi

echo "[OK] preset=${PRESET} aplicado para ring waves_fill"
echo "Recomendado:"
echo "  ./scripts/set-visual.sh ring waves_fill"
echo "  ./scripts/stop.sh && ./scripts/start.sh"
