#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
CFG="./config/base.conf"
TARGET="${1:-}"

if [[ "$TARGET" != "mpvpaper" && "$TARGET" != "layer-shell" ]]; then
  echo "Uso: kitsune output-target <mpvpaper|layer-shell>"
  exit 1
fi

if grep -qE '^[[:space:]]*output_target[[:space:]]*=' "$CFG"; then
  sed -i "s|^[[:space:]]*output_target[[:space:]]*=.*|output_target=$TARGET|" "$CFG"
else
  printf '\noutput_target=%s\n' "$TARGET" >> "$CFG"
fi

echo "[OK] output_target=$TARGET"
echo "[i] Aplica con: kitsune restart"
