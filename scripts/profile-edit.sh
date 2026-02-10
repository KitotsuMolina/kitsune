#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
PARAM="${1:-}"
VALUE_RAW="${2:-}"
FILE_ARG="${3:-}"

if [[ -z "$PARAM" || -z "$VALUE_RAW" ]]; then
  echo "Uso: ./scripts/profile-edit.sh <param> <valor> [archivo.profile]"
  exit 1
fi

if [[ -n "$FILE_ARG" ]]; then
  FILE="$FILE_ARG"
else
  FILE="$(awk -F'=' '$1 ~ /^test_profile_file$/ {print $2}' ./config/base.conf | tr -d '[:space:]')"
  [[ -n "$FILE" ]] || FILE="./config/profiles/test.profile"
fi

if [[ ! -f "$FILE" ]]; then
  touch "$FILE"
fi

CLAMPED="$(node -e '
const p = process.argv[1];
const v = Number(process.argv[2]);
if (!Number.isFinite(v)) process.exit(2);
const lim = {
  low_band_gain:[0,2.5],
  mid_band_gain:[0,2.5],
  high_band_gain:[0,2.5],
  bass_boost:[0,1.5],
  bass_power:[1,4],
  gain:[0,5],
  gamma:[0.0001,3],
  curve_drive:[0.1,3],
  attack:[0,1],
  gravity_step:[0,10],
  avg_frames:[1,30],
  smooth_radius:[0,8],
  height_scale:[0.05,1],
  dune_amount:[0,1],
  dune_cycles:[0.5,6],
  edge_falloff_pow:[0.5,6],
  dune_floor:[0,1],
  dune_softness:[0.3,3],
  twin_amount:[0,1],
  twin_separation:[0.05,0.45],
  twin_width:[0.03,0.35],
  center_dip:[0,1],
  loud_floor:[0,1],
  loud_floor_curve:[0.5,4],
  center_jump_amount:[0,1],
  center_jump_sharpness:[1,20],
  center_jump_threshold:[0.001,1],
  center_jump_decay:[0,1],
  bar_gap:[0,20],
  side_padding:[0,400],
  bottom_padding:[0,400],
  min_bar_height_px:[0,100],
  ring_x:[0,8000],
  ring_y:[0,8000],
  ring_radius:[8,4000],
  ring_thickness:[1,20],
  ring_base_thickness:[1,200],
  ring_bar_thickness:[1,20],
  ring_min_bar:[0,500],
  ring_max_bar:[2,5000],
  silence_timeout_ms:[100,5000]
};
const r = lim[p] || [-1e9, 1e9];
let x = Math.min(r[1], Math.max(r[0], v));
const intParams = new Set(["avg_frames","smooth_radius","bar_gap","side_padding","bottom_padding","min_bar_height_px","ring_x","ring_y","ring_radius","ring_thickness","ring_base_thickness","ring_bar_thickness","silence_timeout_ms"]);
if (intParams.has(p)) x = Math.round(x);
process.stdout.write(String(x));
' "$PARAM" "$VALUE_RAW" 2>/dev/null || true)"

if [[ -z "$CLAMPED" ]]; then
  echo "[!] Valor inválido: $VALUE_RAW"
  exit 1
fi

if rg -n "^${PARAM}=" "$FILE" >/dev/null 2>&1; then
  sed -i "s/^${PARAM}=.*/${PARAM}=${CLAMPED}/" "$FILE"
else
  printf '%s=%s\n' "$PARAM" "$CLAMPED" >> "$FILE"
fi

echo "[OK] $PARAM=$CLAMPED ($FILE)"
