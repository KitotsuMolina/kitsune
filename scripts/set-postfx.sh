#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

ENABLE="${1:-}"
BLUR_PASSES="${2:-}"
BLUR_MIX="${3:-}"
GLOW_STRENGTH="${4:-}"
GLOW_MIX="${5:-}"
SCOPE="${6:-}"

if [[ -z "$ENABLE" || -z "$BLUR_PASSES" || -z "$BLUR_MIX" || -z "$GLOW_STRENGTH" || -z "$GLOW_MIX" ]]; then
  echo "Uso: ./scripts/set-postfx.sh <enable:0|1> <blur_passes:0..4> <blur_mix:0..1> <glow_strength:0..3> <glow_mix:0..1> [scope:final|layer|mixed]"
  exit 1
fi

if ! [[ "$ENABLE" =~ ^[01]$ ]]; then
  echo "[x] enable debe ser 0 o 1"
  exit 1
fi
if ! [[ "$BLUR_PASSES" =~ ^[0-9]+$ ]] || (( BLUR_PASSES < 0 || BLUR_PASSES > 4 )); then
  echo "[x] blur_passes debe estar entre 0 y 4"
  exit 1
fi

check_float_range() {
  local v="$1"
  local min="$2"
  local max="$3"
  awk -v x="$v" -v lo="$min" -v hi="$max" 'BEGIN{ exit !(x+0==x && x>=lo && x<=hi) }'
}

if ! check_float_range "$BLUR_MIX" 0 1; then
  echo "[x] blur_mix debe estar entre 0 y 1"
  exit 1
fi
if ! check_float_range "$GLOW_STRENGTH" 0 3; then
  echo "[x] glow_strength debe estar entre 0 y 3"
  exit 1
fi
if ! check_float_range "$GLOW_MIX" 0 1; then
  echo "[x] glow_mix debe estar entre 0 y 1"
  exit 1
fi

if [[ -n "$SCOPE" && "$SCOPE" != "final" && "$SCOPE" != "layer" && "$SCOPE" != "mixed" ]]; then
  echo "[x] scope debe ser final, layer o mixed"
  exit 1
fi

sed -i "s/^postfx_enabled=.*/postfx_enabled=${ENABLE}/" ./config/base.conf
sed -i "s/^postfx_blur_passes=.*/postfx_blur_passes=${BLUR_PASSES}/" ./config/base.conf
sed -i "s/^postfx_blur_mix=.*/postfx_blur_mix=${BLUR_MIX}/" ./config/base.conf
sed -i "s/^postfx_glow_strength=.*/postfx_glow_strength=${GLOW_STRENGTH}/" ./config/base.conf
sed -i "s/^postfx_glow_mix=.*/postfx_glow_mix=${GLOW_MIX}/" ./config/base.conf
if [[ -n "$SCOPE" ]]; then
  sed -i "s/^postfx_scope=.*/postfx_scope=${SCOPE}/" ./config/base.conf
fi

if [[ -n "$SCOPE" ]]; then
  echo "[OK] postfx_enabled=${ENABLE} scope=${SCOPE} blur_passes=${BLUR_PASSES} blur_mix=${BLUR_MIX} glow_strength=${GLOW_STRENGTH} glow_mix=${GLOW_MIX}"
else
  echo "[OK] postfx_enabled=${ENABLE} blur_passes=${BLUR_PASSES} blur_mix=${BLUR_MIX} glow_strength=${GLOW_STRENGTH} glow_mix=${GLOW_MIX}"
fi
echo "Reinicia: ./scripts/stop.sh && ./scripts/start.sh"
