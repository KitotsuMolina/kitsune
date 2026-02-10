#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

CFG="./config/base.conf"
AUTO_INSTALL=0
INSTALL_COMPLETIONS=0

usage() {
  cat <<'EOF'
Uso:
  ./scripts/install.sh [--install-packages] [--install-completions]

Opciones:
  --install-packages   Intenta instalar dependencias faltantes (requiere sudo).
  --install-completions Instala completions para bash/zsh/fish.

Este script:
  1) valida dependencias
  2) ajusta monitor/resolucion en config/base.conf (si detecta Hyprland)
  3) sincroniza fifo_cava -> config/cava.conf
  4) compila en release
  5) instala comando global 'kitsune' en ~/.local/bin
  6) opcionalmente instala completions de shell
  7) deja listo para usar ./scripts/start.sh
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --install-packages)
      AUTO_INSTALL=1
      shift
      ;;
    --install-completions)
      INSTALL_COMPLETIONS=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[x] Opcion desconocida: $1"
      usage
      exit 1
      ;;
  esac
done

cmd_exists() {
  command -v "$1" >/dev/null 2>&1
}

cfg_get() {
  local key="$1"
  local def="$2"
  local v
  v="$(awk -F'=' -v k="$key" '$1 ~ "^[[:space:]]*"k"[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$CFG" 2>/dev/null || true)"
  if [[ -z "$v" ]]; then
    echo "$def"
  else
    echo "$v"
  fi
}

cfg_set() {
  local key="$1"
  local value="$2"
  local tmp
  tmp="$(mktemp)"
  if awk -F'=' -v k="$key" -v v="$value" '
    BEGIN { done=0 }
    {
      if (!done && $1 ~ "^[[:space:]]*"k"[[:space:]]*$") {
        print k"="v;
        done=1;
      } else {
        print $0;
      }
    }
    END {
      if (!done) print k"="v;
    }
  ' "$CFG" >"$tmp"; then
    mv "$tmp" "$CFG"
  else
    rm -f "$tmp"
    return 1
  fi
}

detect_pkg_manager() {
  if cmd_exists apt-get; then
    echo "apt"
    return
  fi
  if cmd_exists pacman; then
    echo "pacman"
    return
  fi
  if cmd_exists dnf; then
    echo "dnf"
    return
  fi
  if cmd_exists zypper; then
    echo "zypper"
    return
  fi
  echo "unknown"
}

install_packages() {
  local mgr="$1"
  shift
  local pkgs=("$@")
  if [[ "${#pkgs[@]}" -eq 0 ]]; then
    return 0
  fi
  if ! cmd_exists sudo; then
    echo "[x] Falta sudo para instalar paquetes automaticamente"
    return 1
  fi

  case "$mgr" in
    apt)
      sudo apt-get update
      sudo apt-get install -y "${pkgs[@]}"
      ;;
    pacman)
      sudo pacman -Sy --needed "${pkgs[@]}"
      ;;
    dnf)
      sudo dnf install -y "${pkgs[@]}"
      ;;
    zypper)
      sudo zypper install -y "${pkgs[@]}"
      ;;
    *)
      echo "[x] No se detecto un gestor de paquetes soportado"
      return 1
      ;;
  esac
}

install_global_cli() {
  local root_dir
  root_dir="$(pwd)"
  local bin_dir="$HOME/.local/bin"
  local target="$bin_dir/kitsune"
  local linked=0

  mkdir -p "$bin_dir"
  if ln -sfn "$root_dir/scripts/kitsune.sh" "$target" 2>/dev/null; then
    linked=1
  else
    # Fallback si el FS no permite symlink.
    cat >"$target" <<EOF
#!/usr/bin/env bash
exec "$root_dir/scripts/kitsune.sh" "\$@"
EOF
    chmod +x "$target"
    linked=1
  fi

  local path_line='export PATH="$HOME/.local/bin:$PATH"'
  for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
    if [[ -f "$rc" ]]; then
      if ! grep -Fq "$path_line" "$rc"; then
        printf '\n# Added by Kitsune installer\n%s\n' "$path_line" >>"$rc"
      fi
    fi
  done

  if [[ "$linked" -eq 1 ]]; then
    echo "[i] CLI global instalada: $target"
    if [[ ":$PATH:" != *":$bin_dir:"* ]]; then
      echo "[i] Para usarlo en esta sesion actual, ejecuta:"
      echo "    export PATH=\"$HOME/.local/bin:\$PATH\""
    fi
    echo "[i] Luego podras usar: kitsune start | kitsune stop | kitsune visual bars bars_fill"
  fi
}

install_shell_completions() {
  if [[ -x ./scripts/install-completions.sh ]]; then
    echo "[i] Instalando completions de shell..."
    ./scripts/install-completions.sh all
  else
    echo "[!] No existe ./scripts/install-completions.sh, se omite install de completions."
  fi
}

missing_required=()
required_cmds=(cargo rustc cava mpvpaper mpv)
for c in "${required_cmds[@]}"; do
  if ! cmd_exists "$c"; then
    missing_required+=("$c")
  fi
done

if [[ "${#missing_required[@]}" -gt 0 ]]; then
  echo "[!] Faltan dependencias requeridas: ${missing_required[*]}"
  if [[ "$AUTO_INSTALL" -eq 1 ]]; then
    mgr="$(detect_pkg_manager)"
    case "$mgr" in
      apt)
        pkgs=(cargo rustc cava mpv mpvpaper)
        ;;
      pacman)
        pkgs=(rust cava mpv mpvpaper)
        ;;
      dnf)
        pkgs=(rust cargo cava mpv mpvpaper)
        ;;
      zypper)
        pkgs=(rust cargo cava mpv mpvpaper)
        ;;
      *)
        pkgs=()
        ;;
    esac
    if ! install_packages "$mgr" "${pkgs[@]}"; then
      echo "[x] No se pudieron instalar dependencias automaticamente"
      exit 1
    fi
  else
    echo "[i] Ejecuta de nuevo con --install-packages para intentar instalarlas automaticamente."
    exit 1
  fi
fi

if [[ ! -f "$CFG" ]]; then
  echo "[x] No existe $CFG"
  exit 1
fi

mkdir -p ./.run

echo "[i] Detectando monitor y resolucion..."
if cmd_exists hyprctl; then
  MON="$(hyprctl monitors 2>/dev/null | awk '/^Monitor / {print $2; exit}' || true)"
  RES="$(hyprctl monitors 2>/dev/null | awk '/^[[:space:]]*[0-9]+x[0-9]+@/ {print $1; exit}' || true)"
  if [[ -n "${MON:-}" ]]; then
    cfg_set "monitor" "$MON"
    echo "[i] monitor=$MON"
  fi
  if [[ -n "${RES:-}" ]]; then
    W="${RES%%x*}"
    H="${RES#*x}"
    H="${H%%@*}"
    if [[ "$W" =~ ^[0-9]+$ ]] && [[ "$H" =~ ^[0-9]+$ ]]; then
      cfg_set "width" "$W"
      cfg_set "height" "$H"
      echo "[i] resolution=${W}x${H}"
    fi
  fi
else
  echo "[i] hyprctl no disponible, se conserva monitor/resolucion del config"
fi

FIFO_CAVA="$(cfg_get fifo_cava /tmp/cava-rs.raw)"
if [[ -f ./config/cava.conf ]]; then
  sed -i "s|^raw_target = .*|raw_target = ${FIFO_CAVA}|" ./config/cava.conf
fi

DYNAMIC_COLOR="$(cfg_get dynamic_color 0)"
COLOR_FILE="$(cfg_get color_source_file /tmp/kitsune-accent.hex)"
BASE_COLOR="$(cfg_get color '#ff2f8f')"
if [[ "$DYNAMIC_COLOR" == "1" ]]; then
  printf '%s\n' "$BASE_COLOR" > "$COLOR_FILE"
fi

echo "[i] Compilando Kitsune (release)..."
if [[ -f ./Cargo.toml ]]; then
  cargo build --release
else
  BIN_DIR="${KITSUNE_BIN_DIR:-./bin}"
  if [[ -x "$BIN_DIR/kitsune" && -x "$BIN_DIR/kitsune-layer" ]]; then
    echo "[i] Cargo.toml no encontrado; usando binarios empaquetados en $BIN_DIR"
  else
    echo "[x] No hay Cargo.toml y faltan binarios empaquetados en $BIN_DIR"
    echo "[i] Reinstala el paquete AUR o ejecuta install desde un checkout del repo."
    exit 1
  fi
fi

echo "[i] Instalando comando global 'kitsune'..."
install_global_cli
if [[ "$INSTALL_COMPLETIONS" -eq 1 ]]; then
  install_shell_completions
fi

echo "[i] Resumen de estado:"
echo "    - backend:       $(cfg_get backend cpu)"
echo "    - spectrum_mode: $(cfg_get spectrum_mode single)"
echo "    - mode:          $(cfg_get mode bars)"
echo "    - monitor:       $(cfg_get monitor DP-1)"
echo "    - width/height:  $(cfg_get width 1920)x$(cfg_get height 1080)"
echo "    - dynamic_color: $(cfg_get dynamic_color 0)"
echo
echo "[OK] Instalacion/configuracion inicial lista"
echo "     Iniciar: ./scripts/start.sh"
echo "     Detener: ./scripts/stop.sh"
