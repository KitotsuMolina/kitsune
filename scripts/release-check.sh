#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "[release-check] cargo check --bins"
cargo check --bins

echo "[release-check] cli smoke --full"
./tests/cli-smoke.sh --full

echo "[release-check] OK"
