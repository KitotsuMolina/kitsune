#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
SECS="${1:-}"
if ! [[ "$SECS" =~ ^[0-9]+$ ]] || [[ "$SECS" -lt 1 ]]; then
  echo "Uso: ./scripts/set-rotation.sh <segundos>=1.."
  exit 1
fi

sed -i "s/^rotation_seconds=.*/rotation_seconds=${SECS}/" ./config/base.conf

echo "[OK] rotation_seconds=${SECS}"
