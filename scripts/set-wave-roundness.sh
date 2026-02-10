#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
VAL="${1:-}"

if [[ -z "$VAL" ]]; then
  echo "Uso: ./scripts/set-wave-roundness.sh <0.0..1.0>"
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

sed -i "s/^bars_wave_roundness=.*/bars_wave_roundness=${CLAMPED}/" ./config/base.conf
sed -i "s/^ring_wave_roundness=.*/ring_wave_roundness=${CLAMPED}/" ./config/base.conf

echo "[OK] bars_wave_roundness=${CLAMPED}"
echo "[OK] ring_wave_roundness=${CLAMPED}"
echo "Reinicia para aplicar: ./scripts/stop.sh && ./scripts/start.sh"
