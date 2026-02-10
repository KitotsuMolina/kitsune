#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

CFG="${KITSUNE_CFG:-./config/base.conf}"
RUN_PREFIX="${KITSUNE_RUN_PREFIX:-./.run}"
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

FIFO_VIDEO="$(cfg_get fifo_video /tmp/kitsune-spectrum.rgba)"
FIFO_CAVA="$(cfg_get fifo_cava /tmp/cava-rs.raw)"
STOP_TIMEOUT="${KITSUNE_STOP_TIMEOUT:-3}"
FORCE_KILL="${KITSUNE_FORCE_KILL:-1}"

if ! [[ "$STOP_TIMEOUT" =~ ^[0-9]+$ ]] || [[ "$STOP_TIMEOUT" -lt 0 ]]; then
  STOP_TIMEOUT=3
fi

killpid() {
  local f="$1"
  if [[ -f "$f" ]]; then
    local pid
    pid="$(cat "$f" || true)"
    if [[ -n "${pid:-}" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      local waited=0
      while kill -0 "$pid" 2>/dev/null && [[ "$waited" -lt "$STOP_TIMEOUT" ]]; do
        sleep 1
        waited=$((waited + 1))
      done
      if kill -0 "$pid" 2>/dev/null; then
        if [[ "$FORCE_KILL" == "1" ]]; then
          kill -KILL "$pid" 2>/dev/null || true
        else
          echo "[!] pid sigue vivo tras SIGTERM: $pid (KITSUNE_FORCE_KILL=0)"
        fi
      fi
    fi
    rm -f "$f"
  fi
}

echo "[i] Stopping..."
killpid "$PID_REN"
killpid "$PID_MON"
killpid "$PID_CAVA"
killpid "$PID_MPV"
killpid "$PID_LAYER"
killpid "$PID_COLOR"

rm -f "$FIFO_VIDEO" "$FIFO_CAVA" 2>/dev/null || true
rm -f "${RUN_PREFIX}/target_monitor" "${RUN_PREFIX}/target_reason" 2>/dev/null || true

echo "[OK] Stopped"
