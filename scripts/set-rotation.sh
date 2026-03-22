#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
CFG="${KITSUNE_CFG:-./config/base.conf}"
SECS="${1:-}"
if ! [[ "$SECS" =~ ^[0-9]+$ ]] || [[ "$SECS" -lt 1 ]]; then
  echo "Uso: ./scripts/set-rotation.sh <segundos>=1.."
  exit 1
fi

sed -i "s/^rotation_seconds=.*/rotation_seconds=${SECS}/" $CFG

echo "[OK] rotation_seconds=${SECS}"
