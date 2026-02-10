#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

PRESET="${1:-}"
MODE="${2:-}"

if [[ -z "$PRESET" ]]; then
  cat <<USAGE
Uso:
  ./scripts/tune.sh <preset> [bars|ring]

Presets:
  soft
  punchy
  bass-heavy
  vocal
  balanced
  cinematic
  energetic

Ejemplos:
  ./scripts/tune.sh soft ring
  ./scripts/tune.sh bass-heavy bars
USAGE
  exit 1
fi

if [[ -z "$MODE" ]]; then
  MODE="$(awk -F'=' '$1 ~ /^mode$/ {print $2}' ./config/base.conf | tr -d '[:space:]')"
  [[ -n "$MODE" ]] || MODE="ring"
fi

if [[ "$MODE" != "bars" && "$MODE" != "ring" ]]; then
  echo "[!] Modo inválido: $MODE (usa bars|ring)"
  exit 1
fi

# Garantiza test mode para hot-reload
./scripts/set-runtime-mode.sh test >/dev/null

if [[ "$MODE" == "ring" ]]; then
  ./scripts/test-profile-load.sh ring_video_uno >/dev/null
else
  ./scripts/test-profile-load.sh bars_balanced >/dev/null
fi

setv() {
  ./scripts/profile-edit.sh "$1" "$2" >/dev/null
}

apply_common() {
  setv avg_frames 3
  setv smooth_radius 1
  setv attack 0.72
  setv gravity_step 2.9
  setv gain 2.1
  setv gamma 0.70
  setv curve_drive 0.95
  setv bass_boost 0.20
  setv bass_power 2.0
  setv low_band_gain 1.0
  setv mid_band_gain 1.0
  setv high_band_gain 1.0
}

apply_ring_defaults() {
  setv ring_thickness 1
  setv ring_base_thickness 3
  setv ring_bar_thickness 3
  setv ring_min_bar 2
  setv ring_max_bar 175
  setv height_scale 0.52
}

apply_bars_defaults() {
  setv bar_gap 1
  setv side_padding 0
  setv bottom_padding 8
  setv min_bar_height_px 0
  setv height_scale 0.58
}

apply_common
if [[ "$MODE" == "ring" ]]; then
  apply_ring_defaults
else
  apply_bars_defaults
fi

case "$PRESET" in
  soft)
    setv gain 1.85
    setv gamma 0.80
    setv curve_drive 0.88
    setv attack 0.62
    setv gravity_step 2.10
    setv avg_frames 5
    setv smooth_radius 2
    setv bass_boost 0.14
    setv low_band_gain 0.95
    setv mid_band_gain 1.00
    setv high_band_gain 0.90
    setv loud_floor 0.18
    setv loud_floor_curve 1.15
    ;;

  punchy)
    setv gain 2.45
    setv gamma 0.62
    setv curve_drive 1.10
    setv attack 0.80
    setv gravity_step 3.40
    setv avg_frames 2
    setv smooth_radius 0
    setv bass_boost 0.24
    setv low_band_gain 1.15
    setv mid_band_gain 1.00
    setv high_band_gain 1.05
    setv loud_floor 0.10
    setv loud_floor_curve 1.28
    ;;

  bass-heavy)
    setv gain 2.30
    setv gamma 0.66
    setv curve_drive 1.00
    setv attack 0.76
    setv gravity_step 3.00
    setv bass_boost 0.55
    setv bass_power 2.60
    setv low_band_gain 1.55
    setv mid_band_gain 0.95
    setv high_band_gain 0.82
    setv loud_floor 0.14
    setv loud_floor_curve 1.25
    ;;

  vocal)
    setv gain 2.00
    setv gamma 0.72
    setv curve_drive 0.95
    setv attack 0.70
    setv gravity_step 2.70
    setv bass_boost 0.12
    setv bass_power 1.80
    setv low_band_gain 0.82
    setv mid_band_gain 1.30
    setv high_band_gain 1.08
    setv loud_floor 0.12
    setv loud_floor_curve 1.20
    ;;

  balanced)
    # Ya está en defaults comunes
    ;;

  cinematic)
    setv gain 2.10
    setv gamma 0.74
    setv curve_drive 0.92
    setv attack 0.66
    setv gravity_step 2.30
    setv avg_frames 5
    setv smooth_radius 2
    setv bass_boost 0.18
    setv low_band_gain 1.08
    setv mid_band_gain 0.98
    setv high_band_gain 0.88
    setv loud_floor 0.20
    setv loud_floor_curve 1.08
    ;;

  energetic)
    setv gain 2.55
    setv gamma 0.60
    setv curve_drive 1.15
    setv attack 0.84
    setv gravity_step 3.60
    setv avg_frames 2
    setv smooth_radius 0
    setv bass_boost 0.26
    setv low_band_gain 1.22
    setv mid_band_gain 1.04
    setv high_band_gain 1.10
    setv loud_floor 0.09
    setv loud_floor_curve 1.32
    ;;

  *)
    echo "[!] Preset desconocido: $PRESET"
    exit 1
    ;;
esac

echo "[OK] tune aplicado: preset=$PRESET mode=$MODE"
echo "    runtime_mode=test (hot-reload activo)"
echo "    profile=./config/profiles/test.profile"
