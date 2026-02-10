#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

SRC_DIR="./completions"

if [[ ! -d "$SRC_DIR" ]]; then
  echo "[x] No existe directorio de completions: $SRC_DIR"
  exit 1
fi

install_bash() {
  local dst="$HOME/.local/share/bash-completion/completions"
  mkdir -p "$dst"
  cp "$SRC_DIR/kitsune.bash" "$dst/kitsune"
  echo "[OK] bash completion -> $dst/kitsune"
  echo "[i] recarga shell o ejecuta: source ~/.local/share/bash-completion/completions/kitsune"
}

install_zsh() {
  local dst="$HOME/.zsh/completions"
  mkdir -p "$dst"
  cp "$SRC_DIR/_kitsune" "$dst/_kitsune"
  echo "[OK] zsh completion -> $dst/_kitsune"
  echo "[i] asegúrate de tener en ~/.zshrc:"
  echo "    fpath=(~/.zsh/completions \$fpath)"
  echo "    autoload -Uz compinit && compinit"
}

install_fish() {
  local dst="$HOME/.config/fish/completions"
  mkdir -p "$dst"
  cp "$SRC_DIR/kitsune.fish" "$dst/kitsune.fish"
  echo "[OK] fish completion -> $dst/kitsune.fish"
}

TARGET="${1:-all}"
case "$TARGET" in
  all)
    install_bash
    install_zsh
    install_fish
    ;;
  bash)
    install_bash
    ;;
  zsh)
    install_zsh
    ;;
  fish)
    install_fish
    ;;
  *)
    echo "Uso: ./scripts/install-completions.sh [all|bash|zsh|fish]"
    exit 1
    ;;
esac
