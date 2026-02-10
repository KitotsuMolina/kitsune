#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
SECS="${1:-}"
if ! [[ "$SECS" =~ ^[0-9]+$ ]] || [[ "$SECS" -lt 1 ]]; then
  echo "Uso: ./scripts/set-color-poll.sh <segundos>=1.."
  exit 1
fi

sed -i "s/^color_poll_seconds=.*/color_poll_seconds=${SECS}/" ./config/base.conf

echo "[OK] color_poll_seconds=${SECS}"
