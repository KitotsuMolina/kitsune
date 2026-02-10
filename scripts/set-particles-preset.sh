#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
PRESET="${1:-}"

case "$PRESET" in
  off)
    ./scripts/set-particles.sh 0 100 0 0.10 0.20 60 120 1 1 0.50 20
    ;;
  low)
    ./scripts/set-particles.sh 1 320 140 0.10 0.20 80 220 1 2 0.58 36
    ;;
  balanced)
    ./scripts/set-particles.sh 1 520 260 0.12 0.28 95 280 1 2 0.66 42
    ;;
  high)
    ./scripts/set-particles.sh 1 1000 520 0.16 0.42 120 360 1 3 0.76 54
    ;;
  *)
    echo "Uso: ./scripts/set-particles-preset.sh <off|low|balanced|high>"
    exit 1
    ;;
esac
