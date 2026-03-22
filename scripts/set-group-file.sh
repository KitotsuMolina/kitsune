#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
FILE="${1:-}"
GROUPS_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/kitsune/groups"
if [[ -z "$FILE" ]]; then
  echo "Uso: ./scripts/set-group-file.sh <ruta.group>"
  exit 1
fi

mkdir -p "$GROUPS_DIR"
NORMALIZED="${FILE#./}"
if [[ "$NORMALIZED" == config/groups/* ]]; then
  NORMALIZED="${NORMALIZED#config/groups/}"
fi
if [[ "$NORMALIZED" == "$GROUPS_DIR/"* ]]; then
  NORMALIZED="${NORMALIZED#$GROUPS_DIR/}"
fi
if [[ -f "$GROUPS_DIR/$NORMALIZED" ]]; then
  FILE="$NORMALIZED"
fi
sed -i "s|^group_file=.*|group_file=${FILE}|" ./config/base.conf
echo "[OK] group_file=${FILE}"
echo "Reinicia para aplicar: ./scripts/stop.sh && ./scripts/start.sh"
