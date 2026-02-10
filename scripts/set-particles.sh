#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

ENABLE="${1:-}"
MAX="${2:-}"
RATE="${3:-}"
LIFE_MIN="${4:-}"
LIFE_MAX="${5:-}"
SPEED_MIN="${6:-}"
SPEED_MAX="${7:-}"
SIZE_MIN="${8:-}"
SIZE_MAX="${9:-}"
ALPHA="${10:-}"
DRIFT="${11:-}"
SIZE_SCALE="${12:-1.0}"
FADE_JITTER="${13:-0.35}"

if [[ -z "$ENABLE" || -z "$MAX" || -z "$RATE" || -z "$LIFE_MIN" || -z "$LIFE_MAX" || -z "$SPEED_MIN" || -z "$SPEED_MAX" || -z "$SIZE_MIN" || -z "$SIZE_MAX" || -z "$ALPHA" || -z "$DRIFT" ]]; then
  echo "Uso: ./scripts/set-particles.sh <enable:0|1> <max> <rate> <life_min> <life_max> <speed_min> <speed_max> <size_min> <size_max> <alpha:0..1> <drift> [size_scale:0.2..6.0] [fade_jitter:0..1]"
  exit 1
fi

if ! [[ "$ENABLE" =~ ^[01]$ ]]; then
  echo "[x] enable debe ser 0 o 1"
  exit 1
fi
if ! [[ "$MAX" =~ ^[0-9]+$ ]] || (( MAX < 32 || MAX > 20000 )); then
  echo "[x] max debe estar entre 32 y 20000"
  exit 1
fi
if ! [[ "$SIZE_MIN" =~ ^[0-9]+$ && "$SIZE_MAX" =~ ^[0-9]+$ ]]; then
  echo "[x] size_min/size_max deben ser enteros"
  exit 1
fi

check_float_range() {
  local v="$1"
  local min="$2"
  local max="$3"
  awk -v x="$v" -v lo="$min" -v hi="$max" 'BEGIN{ exit !(x+0==x && x>=lo && x<=hi) }'
}

for pair in \
  "$RATE 0 30000" \
  "$LIFE_MIN 0.01 8" \
  "$LIFE_MAX 0.01 8" \
  "$SPEED_MIN 1 4000" \
  "$SPEED_MAX 1 4000" \
  "$ALPHA 0 1" \
  "$DRIFT 0 4000" \
  "$SIZE_SCALE 0.2 6.0" \
  "$FADE_JITTER 0 1"; do
  set -- $pair
  if ! check_float_range "$1" "$2" "$3"; then
    echo "[x] valor fuera de rango: $1 (esperado $2..$3)"
    exit 1
  fi
done

set_key() {
  local key="$1"
  local val="$2"
  if grep -qE "^${key}=" ./config/base.conf; then
    sed -i "s|^${key}=.*|${key}=${val}|" ./config/base.conf
  else
    printf '%s=%s\n' "$key" "$val" >> ./config/base.conf
  fi
}

set_key particles_enabled "$ENABLE"
set_key particles_max "$MAX"
set_key particles_spawn_rate "$RATE"
set_key particles_life_min "$LIFE_MIN"
set_key particles_life_max "$LIFE_MAX"
set_key particles_speed_min "$SPEED_MIN"
set_key particles_speed_max "$SPEED_MAX"
set_key particles_size_min "$SIZE_MIN"
set_key particles_size_max "$SIZE_MAX"
set_key particles_size_scale "$SIZE_SCALE"
set_key particles_alpha "$ALPHA"
set_key particles_drift "$DRIFT"
set_key particles_fade_jitter "$FADE_JITTER"

echo "[OK] particles enabled=${ENABLE} max=${MAX} rate=${RATE} life=${LIFE_MIN}-${LIFE_MAX} speed=${SPEED_MIN}-${SPEED_MAX} size=${SIZE_MIN}-${SIZE_MAX} size_scale=${SIZE_SCALE} alpha=${ALPHA} drift=${DRIFT} fade_jitter=${FADE_JITTER}"
echo "Reinicia: ./scripts/stop.sh && ./scripts/start.sh"
