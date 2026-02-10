#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CLI="$ROOT_DIR/scripts/kitsune.sh"
CFG="$ROOT_DIR/config/base.conf"
CAVA_CFG="$ROOT_DIR/config/cava.conf"
GROUP_DEFAULT="$ROOT_DIR/config/groups/default.group"

MODE="safe"
if [[ "${1:-}" == "--full" ]]; then
  MODE="full"
fi

if [[ ! -x "$CLI" ]]; then
  echo "[x] CLI no ejecutable: $CLI"
  exit 1
fi

TMP_DIR="$(mktemp -d)"
CFG_BAK="$TMP_DIR/base.conf.bak"
CAVA_BAK="$TMP_DIR/cava.conf.bak"
cp "$CFG" "$CFG_BAK"
cp "$CAVA_CFG" "$CAVA_BAK"

restore_config() {
  cp "$CFG_BAK" "$CFG"
  cp "$CAVA_BAK" "$CAVA_CFG"
}

cleanup() {
  restore_config || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

TOTAL=0
PASSED=0
FAILED=0

run_ok() {
  local name="$1"
  shift
  TOTAL=$((TOTAL + 1))
  local out
  local code=0
  out="$("$@" 2>&1)" || code=$?
  if [[ "$code" -eq 0 ]]; then
    PASSED=$((PASSED + 1))
    echo "[ok] $name"
  else
    FAILED=$((FAILED + 1))
    echo "[x] $name"
    if [[ -n "$out" ]]; then
      echo "$out" | sed -n '1,4p'
    fi
  fi
}

run_ok_timeout() {
  local name="$1"
  shift
  TOTAL=$((TOTAL + 1))
  local code=0
  "$@" >/dev/null 2>&1 || code=$?
  if [[ "$code" -eq 0 || "$code" -eq 124 ]]; then
    PASSED=$((PASSED + 1))
    echo "[ok] $name"
  else
    FAILED=$((FAILED + 1))
    echo "[x] $name (exit=$code)"
  fi
}

run_ok_or_skip_permission() {
  local name="$1"
  shift
  TOTAL=$((TOTAL + 1))
  local out
  local code=0
  out="$("$@" 2>&1)" || code=$?
  if [[ "$code" -eq 0 ]]; then
    PASSED=$((PASSED + 1))
    echo "[ok] $name"
    return
  fi
  if grep -Fq "Permission denied" <<< "$out"; then
    PASSED=$((PASSED + 1))
    echo "[ok] $name (skip: permission denied por sandbox)"
    return
  fi
  FAILED=$((FAILED + 1))
  echo "[x] $name"
  if [[ -n "$out" ]]; then
    echo "$out" | sed -n '1,4p'
  fi
}

run_fail() {
  local name="$1"
  shift
  TOTAL=$((TOTAL + 1))
  local code=0
  "$@" >/dev/null 2>&1 || code=$?
  if [[ "$code" -ne 0 ]]; then
    PASSED=$((PASSED + 1))
    echo "[ok] $name"
  else
    FAILED=$((FAILED + 1))
    echo "[x] $name (esperaba fallo y salio 0)"
  fi
}

run_capture() {
  local __var="$1"
  shift
  local out
  out="$($@ 2>/dev/null || true)"
  printf -v "$__var" '%s' "$out"
}

assert_contains() {
  local name="$1"
  local text="$2"
  local needle="$3"
  TOTAL=$((TOTAL + 1))
  if grep -Fq "$needle" <<< "$text"; then
    PASSED=$((PASSED + 1))
    echo "[ok] $name"
  else
    FAILED=$((FAILED + 1))
    echo "[x] $name (no contiene: $needle)"
  fi
}

assert_file_exists() {
  local name="$1"
  local path="$2"
  TOTAL=$((TOTAL + 1))
  if [[ -f "$path" ]]; then
    PASSED=$((PASSED + 1))
    echo "[ok] $name"
  else
    FAILED=$((FAILED + 1))
    echo "[x] $name (falta $path)"
  fi
}

echo "[i] Ejecutando smoke tests ($MODE)"

# Help / discoverability
run_ok "help" "$CLI" help
run_ok "help restart" "$CLI" help restart
run_ok "help config" "$CLI" help config
run_ok "help instances" "$CLI" help instances

# Status / doctor / logs
run_ok "status" "$CLI" status
run_ok "status --all-instances" "$CLI" status --all-instances
run_ok "instances list" "$CLI" instances list
run_fail "instance-status missing" "$CLI" instance-status no-such-monitor
run_ok "doctor" "$CLI" doctor
run_ok "doctor --all-instances" "$CLI" doctor --all-instances
run_ok "layer-status" "$CLI" layer-status
run_ok "logs renderer" "$CLI" logs renderer --lines 5
run_ok "logs all" "$CLI" logs all --lines 2
run_ok "logs layer" "$CLI" logs layer --lines 2
run_ok "logs --all-instances" "$CLI" logs --all-instances --lines 1

# Config commands
run_ok "config list" "$CLI" config list
run_ok "config list --effective" "$CLI" config list --effective
run_ok "config get backend" "$CLI" config get backend

# config set + verify + restore
run_ok "config set backend cpu" "$CLI" config set backend cpu
run_capture backend_after_set "$CLI" config get backend
assert_contains "config backend changed" "$backend_after_set" "cpu"
restore_config

# Visual and render controls (mutan config, luego restauramos)
run_ok "mode bars" "$CLI" mode bars
run_ok "style bars waves" "$CLI" style bars waves
run_ok "visual ring dots" "$CLI" visual ring dots
run_ok "backend gpu" "$CLI" backend gpu
run_ok "output-target mpvpaper" "$CLI" output-target mpvpaper
run_ok "output-target layer-shell" "$CLI" output-target layer-shell
run_ok "output-target mpvpaper (restore)" "$CLI" output-target mpvpaper
run_ok "spectrum-mode single" "$CLI" spectrum-mode single
run_ok "runtime test" "$CLI" runtime test
run_ok "wave-roundness" "$CLI" wave-roundness 0.65
run_ok "ring-fill-softness" "$CLI" ring-fill-softness 0.35
run_ok "waves-fill-preset clean" "$CLI" waves-fill-preset clean
run_ok "postfx" "$CLI" postfx 1 1 0.18 1.2 0.24 mixed
run_ok "particles" "$CLI" particles 1 60 30 0.10 0.20 30 90 1 2 0.5 8
run_ok "particles-look" "$CLI" particles-look back '#FFFFFF'
run_ok "particles-preset low" "$CLI" particles-preset low
run_ok "debug overlay 1" "$CLI" debug overlay 1
restore_config

# Group / profiles / monitor commands
run_ok "group validate default" "$CLI" group validate "$GROUP_DEFAULT"
run_ok "profiles list all" "$CLI" profiles list all
run_ok "profiles list ring" "$CLI" profiles list ring

first_profile="$($CLI profiles list all 2>/dev/null | head -n1 || true)"
if [[ -n "$first_profile" ]]; then
  run_ok "profiles show $first_profile" "$CLI" profiles show "$first_profile"
else
  FAILED=$((FAILED + 1))
  TOTAL=$((TOTAL + 1))
  echo "[x] profiles show (sin perfiles detectados)"
fi

run_ok "profiles set-list bars" "$CLI" profiles set-list bars bars_balanced,bars_punchy
run_ok "profiles set-static ring_video_uno" "$CLI" profiles set-static ring_video_uno
run_ok "profiles rotate on" "$CLI" profiles rotate on
run_ok "profiles rotate off" "$CLI" profiles rotate off

clone_name="zz_smoke_clone_profile"
if [[ -f "$ROOT_DIR/config/profiles/${clone_name}.profile" ]]; then
  rm -f "$ROOT_DIR/config/profiles/${clone_name}.profile"
fi
run_ok "profiles clone" "$CLI" profiles clone bars_balanced "$clone_name"
run_ok "profiles set cloned key" "$CLI" profiles set "$clone_name" gain 2.1
run_ok "profiles show cloned" "$CLI" profiles show "$clone_name"
rm -f "$ROOT_DIR/config/profiles/${clone_name}.profile"
restore_config

tmp_group="$TMP_DIR/group-smoke.group"
cp "$GROUP_DEFAULT" "$tmp_group"
run_ok "group list-layers" "$CLI" group list-layers "$tmp_group"
run_ok "group add-layer" "$CLI" group add-layer "1,bars,bars,bars_balanced,#ffffff,0.40" "$tmp_group"
run_ok "group update-layer 1" "$CLI" group update-layer 1 "1,ring,waves_fill,ring_video_uno,#ff2f8f,0.80" "$tmp_group"
run_ok "group remove-layer 1" "$CLI" group remove-layer 1 "$tmp_group"
run_ok "group validate tmp" "$CLI" group validate "$tmp_group"

run_ok "monitors list" "$CLI" monitors list
current_monitor="$($CLI config get monitor 2>/dev/null || true)"
if [[ -n "$current_monitor" ]]; then
  if command -v hyprctl >/dev/null 2>&1 && hyprctl monitors 2>/dev/null | awk '/^Monitor /{print $2}' | grep -Fxq "$current_monitor"; then
    run_ok "monitor set current" "$CLI" monitor set "$current_monitor"
  else
    TOTAL=$((TOTAL + 1))
    PASSED=$((PASSED + 1))
    echo "[ok] monitor set current (skip: monitor no detectado por hyprctl)"
  fi
fi

# Rotation controls
run_ok "rotate 1" "$CLI" rotate 1
run_ok "rotation 10" "$CLI" rotation 10
run_ok "rotate next" "$CLI" rotate next
run_ok "rotate prev" "$CLI" rotate prev
run_ok "rotate shuffle" "$CLI" rotate shuffle
run_ok "rotate seed 42" "$CLI" rotate seed 42
restore_config

# Color controls
run_ok "dynamic-color 1" "$CLI" dynamic-color 1
run_ok "color-poll 5" "$CLI" color-poll 5
run_ok "colorwatch once" "$CLI" colorwatch "${current_monitor:-DP-1}" /tmp/kitsune-accent.hex 1 --once
restore_config

# Runtime / renderer
run_ok_timeout "run --config (timeout)" timeout 2s "$CLI" run --config "$CFG"

# validate defaults required by reset
assert_file_exists "default base config" "$ROOT_DIR/config/base.conf.default"
assert_file_exists "default cava config" "$ROOT_DIR/config/cava.conf.default"

if [[ "$MODE" == "full" ]]; then
  echo "[i] Ejecutando pruebas full (con efectos de sistema)"
  run_ok "restart" "$CLI" restart
  run_ok "benchmark 2" "$CLI" benchmark 2
  run_ok "autostart status" "$CLI" autostart status
  run_ok "autostart list" "$CLI" autostart list
  run_ok_or_skip_permission "autostart enable" "$CLI" autostart enable
  run_ok_or_skip_permission "autostart enable --monitor" "$CLI" autostart enable --monitor eDP-1
  run_ok "autostart status --monitor" "$CLI" autostart status --monitor eDP-1
  run_ok "autostart disable --monitor" "$CLI" autostart disable --monitor eDP-1
  run_ok "autostart disable" "$CLI" autostart disable
  run_ok "clean" "$CLI" clean
  run_ok "reset" "$CLI" reset
  run_ok "start" "$CLI" start
  run_ok "stop" "$CLI" stop
fi

echo
printf '[i] Resultado: total=%d passed=%d failed=%d\n' "$TOTAL" "$PASSED" "$FAILED"

if [[ "$FAILED" -gt 0 ]]; then
  exit 1
fi
