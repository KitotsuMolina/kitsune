#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

CFG="${KITSUNE_CFG:-./config/base.conf}"
RUN_PREFIX="${KITSUNE_RUN_PREFIX:-./.run}"
LOG_PREFIX="${KITSUNE_LOG_PREFIX:-/tmp/kitsune}"
PID_MPV="${RUN_PREFIX}/mpvpaper.pid"
PID_LAYER="${RUN_PREFIX}/layer.pid"
PID_CAVA="${RUN_PREFIX}/cava.pid"
PID_REN="${RUN_PREFIX}/renderer.pid"
PID_COLOR="${RUN_PREFIX}/colorwatch.pid"
PID_MON="${RUN_PREFIX}/monitorwatch.pid"

cfg_get() {
  local key="$1"
  local def="$2"
  local v
  v="$(awk -F'=' -v k="$key" '$1 ~ "^[[:space:]]*"k"[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$CFG" 2>/dev/null || true)"
  if [[ -z "$v" ]]; then
    echo "$def"
  else
    echo "$v"
  fi
}

WIDTH="$(cfg_get width 1920)"
HEIGHT="$(cfg_get height 1080)"
FPS="$(cfg_get fps 60)"
MONITOR="$(cfg_get monitor DP-1)"
MONITOR_FALLBACK_ENABLED="$(cfg_get monitor_fallback_enabled 1)"
MONITOR_FALLBACK_CHECK_SECONDS="$(cfg_get monitor_fallback_check_seconds 2)"
MONITOR_FALLBACK_PREFER_FOCUSED="$(cfg_get monitor_fallback_prefer_focused 1)"
FIFO_VIDEO="$(cfg_get fifo_video /tmp/kitsune-spectrum.rgba)"
FIFO_CAVA="$(cfg_get fifo_cava /tmp/cava-rs.raw)"
DYNAMIC_COLOR="$(cfg_get dynamic_color 0)"
COLOR_FILE="$(cfg_get color_source_file /tmp/kitsune-accent.hex)"
COLOR_PALETTE_FILE="$(cfg_get color_palette_file /tmp/kitsune-accent.palette)"
COLOR_POLL="$(cfg_get color_poll_seconds 2)"
BASE_COLOR="$(cfg_get color '#ff2f8f')"
OUTPUT_TARGET="$(cfg_get output_target layer-shell)"

mkdir -p "$RUN_PREFIX"
mkdir -p "$(dirname "${LOG_PREFIX}-layer.log")"
: > "${LOG_PREFIX}-layer.log"

if [[ -f "$PID_MPV" || -f "$PID_LAYER" || -f "$PID_CAVA" || -f "$PID_REN" || -f "$PID_COLOR" || -f "$PID_MON" ]]; then
  ./scripts/stop.sh || true
fi

monitor_exists() {
  local mon="$1"
  if ! command -v hyprctl >/dev/null 2>&1; then
    return 1
  fi
  hyprctl monitors 2>/dev/null | awk '/^Monitor /{print $2}' | grep -Fxq "$mon"
}

pick_fallback_monitor() {
  local prefer_focused="$1"
  if ! command -v hyprctl >/dev/null 2>&1; then
    return 1
  fi
  local m=""
  if [[ "$prefer_focused" == "1" ]]; then
    m="$(hyprctl monitors 2>/dev/null | awk '
      /^Monitor / {mon=$2}
      /^[[:space:]]*focused:[[:space:]]*yes/ {if (mon!="") {print mon; exit}}
    ' || true)"
  fi
  if [[ -z "$m" ]]; then
    m="$(hyprctl monitors 2>/dev/null | awk '/^Monitor /{print $2; exit}' || true)"
  fi
  [[ -n "$m" ]] && printf '%s' "$m"
}

resolve_start_monitor() {
  local preferred="$1"
  local fallback_enabled="$2"
  local prefer_focused="$3"
  if [[ "$fallback_enabled" != "1" ]] || ! command -v hyprctl >/dev/null 2>&1; then
    printf '%s' "$preferred"
    return
  fi
  if monitor_exists "$preferred"; then
    printf '%s' "$preferred"
    return
  fi
  local alt
  alt="$(pick_fallback_monitor "$prefer_focused")"
  if [[ -n "$alt" ]]; then
    echo "[i] Monitor '${preferred}' no disponible. Usando fallback '${alt}'." >&2
    printf '%s' "$alt"
    return
  fi
  printf '%s' "$preferred"
}

TARGET_MONITOR="$(resolve_start_monitor "$MONITOR" "$MONITOR_FALLBACK_ENABLED" "$MONITOR_FALLBACK_PREFER_FOCUSED")"
TARGET_REASON="configured"
if [[ "$TARGET_MONITOR" != "$MONITOR" ]]; then
  TARGET_REASON="fallback_from_${MONITOR}"
fi
printf '%s\n' "$TARGET_MONITOR" > "${RUN_PREFIX}/target_monitor"
printf '%s\n' "$TARGET_REASON" > "${RUN_PREFIX}/target_reason"

if [[ "$OUTPUT_TARGET" != "layer-shell" ]]; then
  echo "[x] output_target=$OUTPUT_TARGET ya no es compatible."
  echo "    Usa: kitsune output-target layer-shell"
  exit 1
fi

echo "[i] Building GTK overlay frontend..."
cargo build --release --bin kitsune-overlay

echo "[i] Starting gtk4-layer-shell overlay on ${TARGET_MONITOR}..."
export KITSUNE_CFG="$CFG"
./target/release/kitsune-overlay >"${LOG_PREFIX}-layer.log" 2>&1 &
echo $! > "$PID_LAYER"

if [[ "$DYNAMIC_COLOR" == "1" ]]; then
  printf '%s\n' "$BASE_COLOR" > "$COLOR_FILE"
  cat > "$COLOR_PALETTE_FILE" <<EOF
accent_light=$BASE_COLOR
accent_mid=$BASE_COLOR
accent_dark=$BASE_COLOR
EOF
  if { command -v kitowall >/dev/null 2>&1 || command -v swww >/dev/null 2>&1 || command -v hyprctl >/dev/null 2>&1 || [[ -f "$HOME/.config/hypr/hyprpaper.conf" ]]; } \
    && { command -v magick >/dev/null 2>&1 || command -v convert >/dev/null 2>&1; }; then
    echo "[i] Resolving initial accent color from wallpaper..."
    KITSUNE_PALETTE_FILE="$COLOR_PALETTE_FILE" ./scripts/wallpaper-accent-watcher.sh "$TARGET_MONITOR" "$COLOR_FILE" "$COLOR_POLL" --once >"${LOG_PREFIX}-colorwatch.log" 2>&1 || true
  fi
fi

if [[ "$DYNAMIC_COLOR" == "1" ]]; then
  if { command -v kitowall >/dev/null 2>&1 || command -v swww >/dev/null 2>&1 || command -v hyprctl >/dev/null 2>&1 || [[ -f "$HOME/.config/hypr/hyprpaper.conf" ]]; } \
    && { command -v magick >/dev/null 2>&1 || command -v convert >/dev/null 2>&1; }; then
    echo "[i] Starting wallpaper color watcher..."
    KITSUNE_PALETTE_FILE="$COLOR_PALETTE_FILE" ./scripts/wallpaper-accent-watcher.sh "$TARGET_MONITOR" "$COLOR_FILE" "$COLOR_POLL" >"${LOG_PREFIX}-colorwatch.log" 2>&1 &
    echo $! > "$PID_COLOR"
  else
    echo "[!] dynamic_color=1 pero faltan fuentes de wallpaper (kitowall/swww/hyprctl/hyprpaper.conf) o magick/convert; saltando watcher"
  fi
fi

echo "[OK] Running"
echo "     overlay:  ${LOG_PREFIX}-layer.log"
if [[ -f "$PID_COLOR" ]]; then
  echo "     color:    ${LOG_PREFIX}-colorwatch.log"
fi
