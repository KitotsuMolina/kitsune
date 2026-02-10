#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
SRC_NAME="${1:-}"
if [[ -z "$SRC_NAME" ]]; then
  echo "Uso: ./scripts/test-profile-load.sh <nombre_perfil_sin_extension>"
  exit 1
fi

TEST_FILE="$(awk -F'=' '$1 ~ /^test_profile_file$/ {print $2}' ./config/base.conf | tr -d '[:space:]')"
if [[ -z "$TEST_FILE" ]]; then
  TEST_FILE="./config/profiles/test.profile"
fi

SRC="./config/profiles/${SRC_NAME}.profile"
if [[ ! -f "$SRC" ]]; then
  echo "[!] No existe: $SRC"
  exit 1
fi

cp -f "$SRC" "$TEST_FILE"
echo "[OK] test profile cargado desde: $SRC -> $TEST_FILE"
