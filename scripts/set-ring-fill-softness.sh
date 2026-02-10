#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
VAL="${1:-}"

if [[ -z "$VAL" ]]; then
  echo "Uso: ./scripts/set-ring-fill-softness.sh <0.0..1.0>"
  exit 1
fi

CLAMPED="$(node -e '
const v = Number(process.argv[1]);
if (!Number.isFinite(v)) process.exit(2);
const x = Math.min(1, Math.max(0, v));
process.stdout.write(x.toFixed(2));
' "$VAL" 2>/dev/null || true)"

if [[ -z "$CLAMPED" ]]; then
  echo "[!] Valor invalido: $VAL"
  exit 1
fi

sed -i "s/^ring_fill_softness=.*/ring_fill_softness=${CLAMPED}/" ./config/base.conf
echo "[OK] ring_fill_softness=${CLAMPED}"
echo "Reinicia para aplicar: ./scripts/stop.sh && ./scripts/start.sh"
