#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

CFG="${1:-./config/base.conf}"
PID_MPV="${2:-./.run/mpvpaper.pid}"
PID_COLOR="${3:-./.run/colorwatch.pid}"
PID_MON="${4:-./.run/monitorwatch.pid}"
ACTIVE_MONITOR="${5:-}"
ACTIVE_MON_FILE="./.run/active-monitor"

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

killpid() {
  local f="$1"
  if [[ -f "$f" ]]; then
    local pid
    pid="$(cat "$f" || true)"
    if [[ -n "${pid:-}" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
    rm -f "$f"
  fi
}

pid_alive() {
  local f="$1"
  [[ -f "$f" ]] || return 1
  local pid
  pid="$(cat "$f" || true)"
  [[ -n "${pid:-}" ]] || return 1
  kill -0 "$pid" 2>/dev/null
}

monitor_exists() {
  local mon="$1"
  hyprctl monitors 2>/dev/null | awk '/^Monitor /{print $2}' | grep -Fxq "$mon"
}

pick_fallback_monitor() {
  local prefer_focused="$1"
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

resolve_monitor() {
  local preferred="$1"
  local prefer_focused="$2"
  if monitor_exists "$preferred"; then
    printf '%s' "$preferred"
    return
  fi
  local alt
  alt="$(pick_fallback_monitor "$prefer_focused")"
  if [[ -n "$alt" ]]; then
    printf '%s' "$alt"
    return
  fi
  printf '%s' "$preferred"
}

start_mpvpaper() {
  local mon="$1"
  local width="$2"
  local height="$3"
  local fps="$4"
  local fifo_video="$5"
  mpvpaper --layer bottom \
    -o "no-audio --background=none \
    --untimed \
    --cache=no \
    --cache-secs=0 \
    --demuxer-readahead-secs=0 \
    --demuxer-max-bytes=16MiB \
    --demuxer=rawvideo \
    --demuxer-rawvideo-w=${width} \
    --demuxer-rawvideo-h=${height} \
    --demuxer-rawvideo-fps=${fps} \
    --demuxer-rawvideo-mp-format=rgba" \
    "$mon" "$fifo_video" >/tmp/kitsune-mpvpaper.log 2>&1 &
  echo $! > "$PID_MPV"
}

can_start_colorwatch() {
  { command -v kitowall >/dev/null 2>&1 || command -v swww >/dev/null 2>&1 || command -v hyprctl >/dev/null 2>&1 || [[ -f "$HOME/.config/hypr/hyprpaper.conf" ]]; } \
    && { command -v magick >/dev/null 2>&1 || command -v convert >/dev/null 2>&1; }
}

start_colorwatch() {
  local mon="$1"
  local color_file="$2"
  local color_poll="$3"
  ./scripts/wallpaper-accent-watcher.sh "$mon" "$color_file" "$color_poll" >/tmp/kitsune-colorwatch.log 2>&1 &
  echo $! > "$PID_COLOR"
}

refresh_color_once() {
  local mon="$1"
  local color_file="$2"
  local color_poll="$3"
  ./scripts/wallpaper-accent-watcher.sh "$mon" "$color_file" "$color_poll" --once >/tmp/kitsune-colorwatch.log 2>&1 || true
}

if ! command -v hyprctl >/dev/null 2>&1; then
  echo "[monitorwatch] hyprctl no disponible, saliendo"
  exit 0
fi

WIDTH="$(cfg_get width 1920)"
HEIGHT="$(cfg_get height 1080)"
FPS="$(cfg_get fps 60)"
PREFERRED_MONITOR="$(cfg_get monitor DP-1)"
PREFER_FOCUSED="$(cfg_get monitor_fallback_prefer_focused 1)"
CHECK_SECONDS="$(cfg_get monitor_fallback_check_seconds 2)"
FIFO_VIDEO="$(cfg_get fifo_video /tmp/kitsune-spectrum.rgba)"
DYNAMIC_COLOR="$(cfg_get dynamic_color 0)"
COLOR_FILE="$(cfg_get color_source_file /tmp/kitsune-accent.hex)"
COLOR_POLL="$(cfg_get color_poll_seconds 2)"

if ! [[ "$CHECK_SECONDS" =~ ^[0-9]+$ ]] || [[ "$CHECK_SECONDS" -lt 1 ]]; then
  CHECK_SECONDS=2
fi

if [[ -z "$ACTIVE_MONITOR" ]]; then
  ACTIVE_MONITOR="$(resolve_monitor "$PREFERRED_MONITOR" "$PREFER_FOCUSED")"
fi
printf '%s\n' "$ACTIVE_MONITOR" > "$ACTIVE_MON_FILE"

echo "[monitorwatch] preferred=$PREFERRED_MONITOR active=$ACTIVE_MONITOR check=${CHECK_SECONDS}s"

while true; do
  desired="$(resolve_monitor "$PREFERRED_MONITOR" "$PREFER_FOCUSED")"
  if [[ -z "$desired" ]]; then
    sleep "$CHECK_SECONDS"
    continue
  fi

  if [[ "$desired" != "$ACTIVE_MONITOR" ]] || ! pid_alive "$PID_MPV"; then
    echo "[monitorwatch] switching monitor: ${ACTIVE_MONITOR} -> ${desired}"
    killpid "$PID_MPV"
    start_mpvpaper "$desired" "$WIDTH" "$HEIGHT" "$FPS" "$FIFO_VIDEO"

    if [[ "$DYNAMIC_COLOR" == "1" ]]; then
      killpid "$PID_COLOR"
      if can_start_colorwatch; then
        refresh_color_once "$desired" "$COLOR_FILE" "$COLOR_POLL"
        start_colorwatch "$desired" "$COLOR_FILE" "$COLOR_POLL"
      fi
    fi

    ACTIVE_MONITOR="$desired"
    printf '%s\n' "$ACTIVE_MONITOR" > "$ACTIVE_MON_FILE"
  elif [[ "$DYNAMIC_COLOR" == "1" ]] && ! pid_alive "$PID_COLOR"; then
    if can_start_colorwatch; then
      echo "[monitorwatch] restarting color watcher on $ACTIVE_MONITOR"
      refresh_color_once "$ACTIVE_MONITOR" "$COLOR_FILE" "$COLOR_POLL"
      start_colorwatch "$ACTIVE_MONITOR" "$COLOR_FILE" "$COLOR_POLL"
    fi
  fi

  sleep "$CHECK_SECONDS"
done
