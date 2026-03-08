#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

CFG="./config/base.conf"
CAVA_CFG="./config/cava.conf"
RUN_DIR="./.run"
PID_MPV="$RUN_DIR/mpvpaper.pid"
PID_LAYER="$RUN_DIR/layer.pid"
PID_CAVA="$RUN_DIR/cava.pid"
PID_REN="$RUN_DIR/renderer.pid"
PID_COLOR="$RUN_DIR/colorwatch.pid"
PID_MON="$RUN_DIR/monitorwatch.pid"
SEED_FILE="$RUN_DIR/rotate.seed"
DEFAULT_CFG="./config/base.conf.default"
DEFAULT_CAVA_CFG="./config/cava.conf.default"

LOG_RENDERER="/tmp/kitsune-renderer.log"
LOG_CAVA="/tmp/kitsune-cava.log"
LOG_MPV="/tmp/kitsune-mpvpaper.log"
LOG_LAYER="/tmp/kitsune-layer.log"
LOG_COLOR="/tmp/kitsune-colorwatch.log"
LOG_MON="/tmp/kitsune-monitorwatch.log"

usage() {
  cat <<'USAGE'
Kitsune CLI (entrypoint unico)

Uso:
  kitsune <comando> [args...]
  kitsune help [comando]

Comandos base:
  install [--install-packages]
  start [<monitor>] [--monitor <name>] [--profile <name>|--profiles <p1,p2,...>] [--target <mpvpaper|layer-shell>] [--mode <bars|ring>]
  stop [<monitor>|--monitor <name>]
  restart [--rebuild]
  status [--all-instances]
  doctor [--fix] [--all-instances]
  run [--config <path>]

Observabilidad:
  logs [renderer|cava|layer|mpvpaper|colorwatch|monitorwatch|all] [-f] [--lines N] [--all-instances]
  layer-status

Configuracion:
  config get <key>
  config set <key> <value> [--apply|--restart]
  config list [--effective]

Visual:
  visual <bars|ring> <bars|bars_fill|waves|waves_fill|dots>
  style <bars|ring> <bars|bars_fill|waves|waves_fill|dots>
  mode <bars|ring>
  debug overlay <0|1> [--apply]

Render:
  backend <cpu|gpu>
  output-target <mpvpaper|layer-shell>
  spectrum-mode <single|group>
  group-file <path.group>
  group files
  group create <name|name.group>
  group validate <file.group>
  group list-layers [file.group]
  group add-layer "<csv|layer=csv>" [file.group]
  group update-layer <index> "<csv|layer=csv>" [file.group]
  group remove-layer <index> [file.group]
  runtime <standard|test>

Perfiles:
  rotate <0|1>
  rotate next|prev|shuffle [--apply]
  rotate seed <n>
  rotation <segundos>
  profiles list [bars|ring|all]
  profiles show <name>
  profiles set-list <bars|ring> <p1,p2,...>
  profiles set-static <name>
  profiles rotate <on|off>
  profiles clone <base> <new>
  profiles set <name> <key> <value>
  test-load <profile_name>
  profile-edit <key> <value> [file]
  tune <preset> [bars|ring]

Color:
  dynamic-color <0|1>
  color-poll <segundos>
  colorwatch [monitor] [out_file] [interval] [--once]

PostFX:
  postfx <enable:0|1> <blur_passes:0..4> <blur_mix:0..1> <glow_strength:0..3> <glow_mix:0..1> [scope:final|layer|mixed]

Particulas:
  particles <enable> <max> <rate> <life_min> <life_max> <speed_min> <speed_max> <size_min> <size_max> <alpha> <drift> [size_scale] [fade_jitter]
  particles-look <front|back> <#RRGGBB|spectrum>
  particles-preset <off|low|balanced|high>

Monitores:
  monitors list
  monitor set <name>
  monitor-fallback <enabled:0|1> [check_seconds] [prefer_focused:0|1]

Sistema:
  instances list
  instances status <monitor>
  instance-status <monitor>
  livewallpapers status [--json]
  livewallpapers install [--print]
  autostart enable|disable|status [--monitor <name>]
  autostart list
  autostart list
  clean [--force]
  reset [--restart]
  benchmark [seconds]
USAGE
}

help_cmd() {
  local t="${1:-}"
  case "$t" in
    ""|help|-h|--help)
      usage
      ;;
    restart)
      cat <<'EOFH'
kitsune restart [--rebuild]
- Reinicia stack completo: frontend de salida (mpvpaper o layer-shell) + cava + renderer + watchers.
- Limpia y recrea FIFOs runtime.
- Por defecto recompila (equivalente a stop + start actual).
- --rebuild se acepta por compatibilidad semantica; hoy start ya recompila.
EOFH
      ;;
    logs)
      echo "kitsune logs [renderer|cava|layer|mpvpaper|colorwatch|monitorwatch|all] [-f] [--lines N] [--all-instances]"
      ;;
    layer-status)
      echo "kitsune layer-status"
      ;;
    doctor)
      echo "kitsune doctor [--fix] [--all-instances]"
      ;;
    config)
      cat <<'EOFH'
kitsune config get <key>
kitsune config set <key> <value> [--apply|--restart]
kitsune config list [--effective]
EOFH
      ;;
    instances)
      cat <<'EOFH'
kitsune instances list
kitsune instances status <monitor>
kitsune instance-status <monitor>
EOFH
      ;;
    livewallpapers)
      cat <<'EOFH'
kitsune livewallpapers status [--json]
kitsune livewallpapers install [--print]
- status: valida dependencias requeridas para Workshop/SteamCMD.
- install: intenta instalar dependencias minimas en Arch (steamcmd) o imprime el comando manual con --print.
EOFH
      ;;
    rotate)
      cat <<'EOFH'
kitsune rotate <0|1>
kitsune rotate next|prev|shuffle [--apply]
kitsune rotate seed <n>
EOFH
      ;;
    *)
      echo "No hay help especifico para '$t'."
      ;;
  esac
}

cfg_get() {
  local key="$1"
  local def="$2"
  local v
  v="$(awk -F'=' -v k="$key" '$1 ~ "^[[:space:]]*"k"[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; found=1; exit} END {if (!found) print ""}' "$CFG" 2>/dev/null || true)"
  if [[ -z "$v" ]]; then
    echo "$def"
  else
    echo "$v"
  fi
}

cfg_has_key() {
  local key="$1"
  awk -F'=' -v k="$key" '$1 ~ "^[[:space:]]*"k"[[:space:]]*$" {found=1} END {exit(found?0:1)}' "$CFG"
}

cfg_set() {
  local key="$1"
  local value="$2"
  local tmp
  tmp="$(mktemp)"

  awk -F'=' -v k="$key" -v v="$value" '
    BEGIN {updated=0}
    {
      if ($1 ~ "^[[:space:]]*"k"[[:space:]]*$") {
        print k "=" v
        updated=1
      } else {
        print $0
      }
    }
    END {
      if (!updated) print k "=" v
    }
  ' "$CFG" > "$tmp"

  mv "$tmp" "$CFG"
}

cfg_set_in_file() {
  local file="$1"
  local key="$2"
  local value="$3"
  local tmp
  tmp="$(mktemp)"

  awk -F'=' -v k="$key" -v v="$value" '
    BEGIN {updated=0}
    {
      if ($1 ~ "^[[:space:]]*"k"[[:space:]]*$") {
        print k "=" v
        updated=1
      } else {
        print $0
      }
    }
    END {
      if (!updated) print k "=" v
    }
  ' "$file" > "$tmp"

  mv "$tmp" "$file"
}

cfg_get_in_file() {
  local file="$1"
  local key="$2"
  local def="$3"
  local v
  v="$(awk -F'=' -v k="$key" '$1 ~ "^[[:space:]]*"k"[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; found=1; exit} END {if (!found) print ""}' "$file" 2>/dev/null || true)"
  if [[ -z "$v" ]]; then
    echo "$def"
  else
    echo "$v"
  fi
}

sanitize_instance_id() {
  local raw="$1"
  local out
  out="$(printf '%s' "$raw" | tr -c '[:alnum:]_.-' '_')"
  printf '%s' "$out"
}

instance_id_from_monitor() {
  sanitize_instance_id "$1"
}

instance_root() {
  local id="$1"
  printf '%s/instances/%s' "$RUN_DIR" "$id"
}

instance_cfg_path() {
  local id="$1"
  printf '%s/config/base.conf' "$(instance_root "$id")"
}

instance_run_prefix() {
  local id="$1"
  printf '%s/run' "$(instance_root "$id")"
}

instance_log_prefix() {
  local id="$1"
  printf '/tmp/kitsune-%s' "$id"
}

instance_ids_all() {
  if [[ -d "$RUN_DIR/instances" ]]; then
    local d
    for d in "$RUN_DIR"/instances/*; do
      [[ -d "$d" ]] || continue
      basename "$d"
    done
  fi
}

instance_monitor_from_cfg() {
  local cfg="$1"
  awk -F= '$1 ~ "^[[:space:]]*monitor[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg" 2>/dev/null || true
}

resolve_node_bin() {
  if command -v node >/dev/null 2>&1; then
    command -v node
    return 0
  fi
  if command -v nodejs >/dev/null 2>&1; then
    command -v nodejs
    return 0
  fi
  if [[ -d "$HOME/.nvm/versions/node" ]]; then
    local nbin
    nbin="$(ls -1d "$HOME/.nvm/versions/node"/*/bin/node 2>/dev/null | sort -V | tail -n1 || true)"
    if [[ -n "$nbin" && -x "$nbin" ]]; then
      echo "$nbin"
      return 0
    fi
  fi
  return 1
}

resolve_kitowall_cli_path() {
  local candidates=(
    "$HOME/Programacion/Personal/Wallpaper/Kitowall/dist/cli.js"
    "$HOME/Programacion/Personal/Wallpaper/hyprwall/dist/cli.js"
    "$HOME/.local/share/kitowall/dist/cli.js"
  )
  local c
  for c in "${candidates[@]}"; do
    if [[ -f "$c" ]]; then
      echo "$c"
      return 0
    fi
  done
  return 1
}

resolve_steamcmd_install_cmd() {
  # Prefer official repos when available.
  if command -v pacman >/dev/null 2>&1; then
    if pacman -Si steamcmd >/dev/null 2>&1; then
      echo "sudo pacman -S --needed steamcmd"
      return 0
    fi
  fi
  # Fallback to AUR helpers.
  if command -v yay >/dev/null 2>&1; then
    echo "yay -S --needed steamcmd"
    return 0
  fi
  if command -v paru >/dev/null 2>&1; then
    echo "paru -S --needed steamcmd"
    return 0
  fi
  return 1
}

resolve_livewallpapers_install_cmd() {
  local helper=""
  local pkgs=(steamcmd)
  local joined="${pkgs[*]}"
  joined="${joined// / }"

  if command -v yay >/dev/null 2>&1; then
    helper="yay"
  elif command -v paru >/dev/null 2>&1; then
    helper="paru"
  fi

  if [[ -n "$helper" ]]; then
    echo "$helper -S --needed $joined"
    return 0
  fi

  if command -v pacman >/dev/null 2>&1; then
    if pacman -Si steamcmd >/dev/null 2>&1; then
      echo "sudo pacman -S --needed steamcmd"
      return 0
    fi
  fi

  return 1
}

kitowall_status_payload() {
  local node_bin="$1"
  if command -v kitowall >/dev/null 2>&1; then
    timeout 4 kitowall status 2>/dev/null
    return $?
  fi
  local cli_path
  cli_path="$(resolve_kitowall_cli_path || true)"
  if [[ -n "$cli_path" ]]; then
    timeout 4 "$node_bin" "$cli_path" status 2>/dev/null
    return $?
  fi
  return 127
}

kitowall_contract_check() {
  local monitor="$1"
  local node_bin
  node_bin="$(resolve_node_bin || true)"

  if [[ -z "$node_bin" ]]; then
    echo "[x] node no esta disponible (PATH ni nvm) para validar JSON de kitowall status"
    return 1
  fi
  if ! command -v kitowall >/dev/null 2>&1 && ! resolve_kitowall_cli_path >/dev/null 2>&1; then
    echo "[x] kitowall no esta en PATH y no se encontro dist/cli.js local"
    return 1
  fi

  local payload
  if ! payload="$(kitowall_status_payload "$node_bin")"; then
    echo "[x] kitowall status fallo (exit != 0)"
    return 1
  fi

  if ! printf '%s' "$payload" | "$node_bin" -e '
    let s = "";
    process.stdin.on("data", d => s += d);
    process.stdin.on("end", () => {
      const mon = process.argv[1];
      try {
        const j = JSON.parse(s || "{}");
        const last = j.last_set;
        if (!last || typeof last !== "object") process.exit(12);
        if (typeof last[mon] === "string") process.exit(0);
        const alt = Object.keys(last).find(k => k.toLowerCase() === String(mon).toLowerCase());
        if (alt && typeof last[alt] === "string") process.exit(0);
        process.exit(13);
      } catch {
        process.exit(11);
      }
    });
  ' "$monitor"; then
    local ec=$?
    case "$ec" in
      11) echo "[x] kitowall status devolvio JSON invalido" ;;
      12) echo "[x] kitowall status no contiene objeto last_set" ;;
      13) echo "[x] kitowall status no tiene entrada para monitor '$monitor' en last_set" ;;
      *) echo "[x] validacion de contrato kitowall fallo (code=$ec)" ;;
    esac
    return 1
  fi
  echo "[ok] contrato kitowall status: monitor '$monitor' encontrado en last_set"
  return 0
}

infer_mode_from_profile() {
  local p="$1"
  case "$p" in
    bars*) echo "bars" ;;
    ring*) echo "ring" ;;
    *) echo "" ;;
  esac
}

validate_profile_exists() {
  local p="$1"
  [[ -f "./config/profiles/${p}.profile" ]]
}

cmd_start() {
  local monitor=""
  local profile=""
  local profiles=""
  local target=""
  local mode=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --monitor)
        monitor="${2:-}"
        shift
        ;;
      --profile)
        profile="${2:-}"
        shift
        ;;
      --profiles)
        profiles="${2:-}"
        shift
        ;;
      --target)
        target="${2:-}"
        shift
        ;;
      --mode)
        mode="${2:-}"
        shift
        ;;
      *)
        if [[ -z "$monitor" ]]; then
          monitor="$1"
        else
          echo "[x] Argumento desconocido: $1"
          exit 1
        fi
        ;;
    esac
    shift
  done

  if [[ -z "$monitor" && -z "$profile" && -z "$profiles" && -z "$target" && -z "$mode" ]]; then
    ./scripts/start.sh
    return
  fi

  if [[ -z "$monitor" ]]; then
    echo "[x] Debes indicar monitor para start por instancia."
    echo "Uso: kitsune start <monitor> [--profile <name>|--profiles <p1,p2>] [--target <mpvpaper|layer-shell>] [--mode <bars|ring>]"
    exit 1
  fi

  if [[ -n "$profile" && -n "$profiles" ]]; then
    echo "[x] Usa --profile o --profiles, no ambos."
    exit 1
  fi

  if [[ -n "$target" && "$target" != "mpvpaper" && "$target" != "layer-shell" ]]; then
    echo "[x] target invalido: $target (usa mpvpaper|layer-shell)"
    exit 1
  fi

  if [[ -n "$mode" && "$mode" != "bars" && "$mode" != "ring" ]]; then
    echo "[x] mode invalido: $mode (usa bars|ring)"
    exit 1
  fi

  local id root cfg_inst run_pref log_pref cava_inst
  id="$(instance_id_from_monitor "$monitor")"
  root="$(instance_root "$id")"
  cfg_inst="$(instance_cfg_path "$id")"
  run_pref="$(instance_run_prefix "$id")"
  log_pref="$(instance_log_prefix "$id")"
  cava_inst="$root/config/cava.conf"

  mkdir -p "$root/config" "$run_pref"
  cp "$CFG" "$cfg_inst"
  cp "$CAVA_CFG" "$cava_inst"

  cfg_set_in_file "$cfg_inst" monitor "$monitor"
  cfg_set_in_file "$cfg_inst" fifo_video "/tmp/kitsune-spectrum-${id}.rgba"
  cfg_set_in_file "$cfg_inst" fifo_cava "/tmp/cava-rs-${id}.raw"
  cfg_set_in_file "$cfg_inst" color_source_file "/tmp/kitsune-accent-${id}.hex"
  cfg_set_in_file "$cfg_inst" monitor_fallback_enabled "0"

  if [[ -n "$target" ]]; then
    cfg_set_in_file "$cfg_inst" output_target "$target"
  fi
  local effective_target
  if [[ -n "$target" ]]; then
    effective_target="$target"
  else
    effective_target="$(awk -F= '$1 ~ "^[[:space:]]*output_target[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg_inst")"
    [[ -n "$effective_target" ]] || effective_target="mpvpaper"
  fi

  if command -v hyprctl >/dev/null 2>&1; then
    local mon_list
    mon_list="$(hyprctl monitors 2>/dev/null | awk '/^Monitor /{print $2}' || true)"
    if [[ -n "$mon_list" ]]; then
      if ! grep -Fxq "$monitor" <<< "$mon_list"; then
        echo "[x] Monitor '$monitor' no detectado por hyprctl."
        echo "    Usa: kitsune monitors list"
        exit 1
      fi
    else
      echo "[!] hyprctl sin salida: no se pudo validar monitor '$monitor'. Continuando."
    fi
  elif [[ "$effective_target" == "mpvpaper" ]]; then
    echo "[!] hyprctl no disponible: no se pudo validar monitor '$monitor' para mpvpaper."
  fi

  local selected_mode="$mode"
  if [[ -n "$profile" ]]; then
    if ! validate_profile_exists "$profile"; then
      echo "[x] Perfil no existe: $profile"
      exit 1
    fi
    if [[ -z "$selected_mode" ]]; then
      selected_mode="$(infer_mode_from_profile "$profile")"
    fi
    cfg_set_in_file "$cfg_inst" static_profile "$profile"
    cfg_set_in_file "$cfg_inst" rotate_profiles "0"
  elif [[ -n "$profiles" ]]; then
    IFS=',' read -r -a arr <<< "$profiles"
    if [[ "${#arr[@]}" -eq 0 ]]; then
      echo "[x] --profiles vacio"
      exit 1
    fi
    local first=""
    local inferred=""
    local p norm
    local list_join=""
    for p in "${arr[@]}"; do
      norm="$(printf '%s' "$p" | xargs)"
      if [[ -z "$norm" ]]; then
        continue
      fi
      if ! validate_profile_exists "$norm"; then
        echo "[x] Perfil no existe: $norm"
        exit 1
      fi
      if [[ -z "$first" ]]; then
        first="$norm"
        inferred="$(infer_mode_from_profile "$norm")"
      fi
      if [[ -z "$list_join" ]]; then
        list_join="$norm"
      else
        list_join="${list_join},${norm}"
      fi
    done
    if [[ -z "$first" ]]; then
      echo "[x] --profiles no contiene perfiles validos"
      exit 1
    fi
    if [[ -z "$selected_mode" ]]; then
      selected_mode="$inferred"
    fi
    cfg_set_in_file "$cfg_inst" static_profile "$first"
    cfg_set_in_file "$cfg_inst" rotate_profiles "1"
    if [[ "$selected_mode" == "ring" ]]; then
      cfg_set_in_file "$cfg_inst" ring_profiles "$list_join"
    else
      cfg_set_in_file "$cfg_inst" bars_profiles "$list_join"
    fi
  fi

  if [[ -n "$selected_mode" ]]; then
    cfg_set_in_file "$cfg_inst" mode "$selected_mode"
  fi

  echo "[i] Starting instance id=$id monitor=$monitor"
  echo "[i] config=$cfg_inst"
  KITSUNE_CFG="$cfg_inst" KITSUNE_CAVA_CFG="$cava_inst" KITSUNE_RUN_PREFIX="$run_pref" KITSUNE_LOG_PREFIX="$log_pref" ./scripts/start.sh
}

cmd_stop() {
  local monitor=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --monitor)
        monitor="${2:-}"
        shift
        ;;
      *)
        if [[ -z "$monitor" ]]; then
          monitor="$1"
        else
          echo "[x] Argumento desconocido: $1"
          exit 1
        fi
        ;;
    esac
    shift
  done

  if [[ -z "$monitor" ]]; then
    ./scripts/stop.sh
    return
  fi

  local id cfg_inst run_pref
  id="$(instance_id_from_monitor "$monitor")"
  cfg_inst="$(instance_cfg_path "$id")"
  run_pref="$(instance_run_prefix "$id")"

  if [[ ! -f "$cfg_inst" ]]; then
    echo "[x] No existe instancia para monitor '$monitor' (id=$id)"
    exit 1
  fi

  echo "[i] Stopping instance id=$id monitor=$monitor"
  KITSUNE_CFG="$cfg_inst" KITSUNE_RUN_PREFIX="$run_pref" ./scripts/stop.sh
}

print_instance_status() {
  local id="$1"
  local cfg_inst run_pref log_pref
  cfg_inst="$(instance_cfg_path "$id")"
  run_pref="$(instance_run_prefix "$id")"
  log_pref="$(instance_log_prefix "$id")"

  local cfg_mon cfg_target cfg_mode cfg_runtime cfg_static cfg_backend cfg_bars_style cfg_ring_style cfg_rotate cfg_test
  cfg_mon="$(awk -F= '$1 ~ "^[[:space:]]*monitor[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg_inst" 2>/dev/null || true)"
  cfg_target="$(awk -F= '$1 ~ "^[[:space:]]*output_target[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg_inst" 2>/dev/null || true)"
  cfg_mode="$(awk -F= '$1 ~ "^[[:space:]]*mode[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg_inst" 2>/dev/null || true)"
  cfg_runtime="$(awk -F= '$1 ~ "^[[:space:]]*runtime_mode[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg_inst" 2>/dev/null || true)"
  cfg_static="$(awk -F= '$1 ~ "^[[:space:]]*static_profile[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg_inst" 2>/dev/null || true)"
  cfg_backend="$(awk -F= '$1 ~ "^[[:space:]]*backend[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg_inst" 2>/dev/null || true)"
  cfg_bars_style="$(awk -F= '$1 ~ "^[[:space:]]*bars_style[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg_inst" 2>/dev/null || true)"
  cfg_ring_style="$(awk -F= '$1 ~ "^[[:space:]]*ring_style[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg_inst" 2>/dev/null || true)"
  cfg_rotate="$(awk -F= '$1 ~ "^[[:space:]]*rotate_profiles[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg_inst" 2>/dev/null || true)"
  cfg_test="$(awk -F= '$1 ~ "^[[:space:]]*test_profile_file[[:space:]]*$" {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$cfg_inst" 2>/dev/null || true)"

  local r c m l state="DOWN"
  r="$(pid_state "$run_pref/renderer.pid")"
  c="$(pid_state "$run_pref/cava.pid")"
  m="$(pid_state "$run_pref/mpvpaper.pid")"
  l="$(pid_state "$run_pref/layer.pid")"
  if [[ "$r" == running:* && "$c" == running:* && ( "$m" == running:* || "$l" == running:* ) ]]; then
    state="UP"
  elif [[ "$r" == running:* || "$c" == running:* || "$m" == running:* || "$l" == running:* ]]; then
    state="PARTIAL"
  fi

  local real_mon reason
  real_mon="$(cat "$run_pref/target_monitor" 2>/dev/null || true)"
  reason="$(cat "$run_pref/target_reason" 2>/dev/null || true)"

  local style active_style
  if [[ "$cfg_mode" == "ring" ]]; then
    active_style="$cfg_ring_style"
  else
    active_style="$cfg_bars_style"
  fi
  style="${active_style:-n/a}"

  local last_error=""
  local err
  for err in "${log_pref}-renderer.log" "${log_pref}-layer.log" "${log_pref}-mpvpaper.log" "${log_pref}-cava.log"; do
    if [[ -f "$err" ]]; then
      last_error="$(grep -E 'Error:|\\[.*\\].*error|failed|panic' "$err" 2>/dev/null | tail -n1 || true)"
      [[ -n "$last_error" ]] && break
    fi
  done

  echo "Instance status:"
  echo "  id=$id"
  echo "  monitor_selected=${cfg_mon:-?}"
  echo "  monitor_real=${real_mon:-n/a}"
  echo "  monitor_reason=${reason:-n/a}"
  echo "  state=$state"
  echo "  output_target=${cfg_target:-mpvpaper}"
  echo "  backend=${cfg_backend:-cpu}"
  echo "  mode=${cfg_mode:-bars}"
  echo "  style=$style"
  echo "  runtime_mode=${cfg_runtime:-standard}"
  echo "  rotate_profiles=${cfg_rotate:-0}"
  echo "  static_profile=${cfg_static:-n/a}"
  echo "  test_profile_file=${cfg_test:-n/a}"
  echo "  config=$cfg_inst"
  echo "  run_prefix=$run_pref"
  if [[ -n "$last_error" ]]; then
    echo "  last_error=$last_error"
  fi
  echo "PIDs:"
  echo "  renderer=$(pid_state "$run_pref/renderer.pid")"
  echo "  cava=$(pid_state "$run_pref/cava.pid")"
  echo "  mpvpaper=$(pid_state "$run_pref/mpvpaper.pid")"
  echo "  layer=$(pid_state "$run_pref/layer.pid")"
  echo "  colorwatch=$(pid_state "$run_pref/colorwatch.pid")"
  echo "  monitorwatch=$(pid_state "$run_pref/monitorwatch.pid")"
  echo "Logs:"
  echo "  renderer=${log_pref}-renderer.log"
  echo "  cava=${log_pref}-cava.log"
  echo "  mpvpaper=${log_pref}-mpvpaper.log"
  echo "  layer=${log_pref}-layer.log"
  echo "  colorwatch=${log_pref}-colorwatch.log"
  echo "  monitorwatch=${log_pref}-monitorwatch.log"
}

cmd_instances() {
  local sub="${1:-list}"
  case "$sub" in
    list)
      local any=0 id cfg_inst mon run_pref r c m l state
      for id in $(instance_ids_all); do
        any=1
        cfg_inst="$(instance_cfg_path "$id")"
        run_pref="$(instance_run_prefix "$id")"
        mon="$(instance_monitor_from_cfg "$cfg_inst")"
        r="$(pid_state "$run_pref/renderer.pid")"
        c="$(pid_state "$run_pref/cava.pid")"
        m="$(pid_state "$run_pref/mpvpaper.pid")"
        l="$(pid_state "$run_pref/layer.pid")"
        state="down"
        if [[ "$r" == running:* && "$c" == running:* && ( "$m" == running:* || "$l" == running:* ) ]]; then
          state="up"
        elif [[ "$r" == running:* || "$c" == running:* || "$m" == running:* || "$l" == running:* ]]; then
          state="partial"
        fi
        echo "id=$id monitor=${mon:-?} state=$state"
      done
      if [[ "$any" -eq 0 ]]; then
        echo "(sin instancias)"
      fi
      ;;
    status)
      local monitor="${2:-}"
      if [[ -z "$monitor" ]]; then
        echo "Uso: kitsune instances status <monitor>"
        exit 1
      fi
      local id cfg_inst
      id="$(instance_id_from_monitor "$monitor")"
      cfg_inst="$(instance_cfg_path "$id")"
      if [[ ! -f "$cfg_inst" ]]; then
        echo "[x] No existe instancia para monitor '$monitor' (id=$id)"
        exit 1
      fi
      print_instance_status "$id"
      ;;
    *)
      echo "Uso: kitsune instances <list|status>"
      exit 1
      ;;
  esac
}

pid_state() {
  local f="$1"
  if [[ ! -f "$f" ]]; then
    echo "missing"
    return
  fi
  local pid
  pid="$(cat "$f" 2>/dev/null || true)"
  if [[ -z "${pid:-}" ]]; then
    echo "stale"
    return
  fi
  if kill -0 "$pid" 2>/dev/null; then
    echo "running:$pid"
  else
    echo "stale:$pid"
  fi
}

stack_is_running() {
  local r c m l
  r="$(pid_state "$PID_REN")"
  c="$(pid_state "$PID_CAVA")"
  m="$(pid_state "$PID_MPV")"
  l="$(pid_state "$PID_LAYER")"
  [[ "$r" == running:* && "$c" == running:* && ( "$m" == running:* || "$l" == running:* ) ]]
}

print_status() {
  local backend spectrum mode bars_style ring_style runtime mon rot rot_sec dyn fifo_v fifo_c group_file output_target
  backend="$(cfg_get backend cpu)"
  spectrum="$(cfg_get spectrum_mode single)"
  mode="$(cfg_get mode bars)"
  bars_style="$(cfg_get bars_style bars)"
  ring_style="$(cfg_get ring_style bars)"
  runtime="$(cfg_get runtime_mode standard)"
  mon="$(cfg_get monitor DP-1)"
  rot="$(cfg_get rotate_profiles 0)"
  rot_sec="$(cfg_get rotation_seconds 10)"
  dyn="$(cfg_get dynamic_color 0)"
  fifo_v="$(cfg_get fifo_video /tmp/kitsune-spectrum.rgba)"
  fifo_c="$(cfg_get fifo_cava /tmp/cava-rs.raw)"
  group_file="$(cfg_get group_file ./config/groups/default.group)"
  output_target="$(cfg_get output_target mpvpaper)"
  local real_mon reason
  real_mon="$(cat "$RUN_DIR/target_monitor" 2>/dev/null || true)"
  reason="$(cat "$RUN_DIR/target_reason" 2>/dev/null || true)"

  if stack_is_running; then
    echo "Stack: UP"
  else
    echo "Stack: DOWN/PARTIAL"
  fi

  echo "Config:"
  echo "  backend=$backend"
  echo "  spectrum_mode=$spectrum"
  echo "  mode=$mode"
  echo "  bars_style=$bars_style"
  echo "  ring_style=$ring_style"
  echo "  runtime_mode=$runtime"
  echo "  monitor=$mon"
  echo "  monitor_real=${real_mon:-n/a}"
  echo "  monitor_reason=${reason:-n/a}"
  echo "  output_target=$output_target"
  echo "  group_file=$group_file"
  echo "  rotate_profiles=$rot"
  echo "  rotation_seconds=$rot_sec"
  echo "  dynamic_color=$dyn"
  echo "  fifo_video=$fifo_v"
  echo "  fifo_cava=$fifo_c"

  echo "PIDs:"
  local f b s
  for f in "$PID_REN" "$PID_CAVA" "$PID_MPV" "$PID_LAYER" "$PID_COLOR" "$PID_MON"; do
    b="$(basename "$f")"
    s="$(pid_state "$f")"
    case "$s" in
      running:*) echo "  $b: ${s#running:} (running)" ;;
      stale:*) echo "  $b: ${s#stale:} (stale)" ;;
      stale) echo "  $b: stale" ;;
      *) echo "  $b: missing" ;;
    esac
  done

  echo "Logs:"
  echo "  renderer=$LOG_RENDERER"
  echo "  cava=$LOG_CAVA"
  echo "  mpvpaper=$LOG_MPV"
  echo "  layer=$LOG_LAYER"
  echo "  colorwatch=$LOG_COLOR"
  echo "  monitorwatch=$LOG_MON"

  local fps
  fps="$(grep -Eo 'fps[=: ]+[0-9]+(\.[0-9]+)?' "$LOG_RENDERER" 2>/dev/null | tail -n1 | grep -Eo '[0-9]+(\.[0-9]+)?' || true)"
  if [[ -n "$fps" ]]; then
    echo "Runtime metrics:"
    echo "  fps_real=$fps"
    awk -v f="$fps" 'BEGIN {if (f>0) printf "  frame_ms=%.2f\n", (1000/f)}'
  else
    echo "Runtime metrics:"
    echo "  fps_real=n/a (renderer log sin trazas fps)"
  fi
}

cmd_status() {
  local all_instances=0
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --all-instances) all_instances=1 ;;
      *)
        echo "[x] Opcion desconocida para status: $1"
        exit 1
        ;;
    esac
    shift
  done

  print_status
  if [[ "$all_instances" == "1" ]]; then
    echo
    echo "All instances:"
    local any=0 id
    for id in $(instance_ids_all); do
      any=1
      echo "--- id=$id ---"
      print_instance_status "$id"
    done
    if [[ "$any" -eq 0 ]]; then
      echo "(sin instancias)"
    fi
  fi
}

cmd_logs() {
  local src="all"
  local follow=0
  local lines=120
  local all_instances=0
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -f|--follow)
        follow=1
        ;;
      --lines)
        lines="${2:-120}"
        shift
        ;;
      --all-instances)
        all_instances=1
        ;;
      renderer|cava|mpvpaper|layer|colorwatch|monitorwatch|all)
        src="$1"
        ;;
      *)
        echo "[x] Opcion desconocida para logs: $1"
        exit 1
        ;;
    esac
    shift
  done

  if [[ "$all_instances" == "1" ]]; then
    local any=0 id log_pref files inst_files f
    for id in $(instance_ids_all); do
      any=1
      log_pref="$(instance_log_prefix "$id")"
      inst_files=()
      case "$src" in
        renderer) inst_files+=("${log_pref}-renderer.log") ;;
        cava) inst_files+=("${log_pref}-cava.log") ;;
        mpvpaper) inst_files+=("${log_pref}-mpvpaper.log") ;;
        layer) inst_files+=("${log_pref}-layer.log") ;;
        colorwatch) inst_files+=("${log_pref}-colorwatch.log") ;;
        monitorwatch) inst_files+=("${log_pref}-monitorwatch.log") ;;
        all) inst_files+=("${log_pref}-renderer.log" "${log_pref}-cava.log" "${log_pref}-mpvpaper.log" "${log_pref}-layer.log" "${log_pref}-colorwatch.log" "${log_pref}-monitorwatch.log") ;;
      esac
      echo "===== instance:$id ====="
      if [[ "$follow" == "1" ]]; then
        tail -n "$lines" -f "${inst_files[@]}"
        return
      fi
      for f in "${inst_files[@]}"; do
        echo "--- $f"
        if [[ -f "$f" ]]; then
          tail -n "$lines" "$f"
        else
          echo "(no existe)"
        fi
      done
    done
    if [[ "$any" -eq 0 ]]; then
      echo "(sin instancias)"
    fi
    return
  fi

  local files=()
  case "$src" in
    renderer) files+=("$LOG_RENDERER") ;;
    cava) files+=("$LOG_CAVA") ;;
    mpvpaper) files+=("$LOG_MPV") ;;
    layer) files+=("$LOG_LAYER") ;;
    colorwatch) files+=("$LOG_COLOR") ;;
    monitorwatch) files+=("$LOG_MON") ;;
    all) files+=("$LOG_RENDERER" "$LOG_CAVA" "$LOG_MPV" "$LOG_LAYER" "$LOG_COLOR" "$LOG_MON") ;;
    *)
      echo "[x] Source invalido: $src"
      exit 1
      ;;
  esac

  if [[ "$follow" == "1" ]]; then
    tail -n "$lines" -f "${files[@]}"
  else
    local f
    for f in "${files[@]}"; do
      echo "===== $f ====="
      if [[ -f "$f" ]]; then
        tail -n "$lines" "$f"
      else
        echo "(no existe)"
      fi
    done
  fi
}

cmd_layer_status() {
  local output_target monitor pid_state_line
  output_target="$(cfg_get output_target mpvpaper)"
  monitor="$(cfg_get monitor DP-1)"
  pid_state_line="$(pid_state "$PID_LAYER")"

  echo "Layer status:"
  echo "  output_target=$output_target"
  echo "  monitor(config)=$monitor"
  case "$pid_state_line" in
    running:*) echo "  layer_pid=${pid_state_line#running:} (running)" ;;
    stale:*) echo "  layer_pid=${pid_state_line#stale:} (stale)" ;;
    stale) echo "  layer_pid=stale" ;;
    *) echo "  layer_pid=missing" ;;
  esac

  if [[ ! -f "$LOG_LAYER" ]]; then
    echo "  layer_log=$LOG_LAYER (missing)"
    return 0
  fi

  echo "  layer_log=$LOG_LAYER"
  local selected fallback configured last_err
  selected="$(grep -F '[layer] selected output by monitor name:' "$LOG_LAYER" | tail -n1 || true)"
  fallback="$(grep -F "[layer] monitor '" "$LOG_LAYER" | grep -F 'not found; using compositor default output' | tail -n1 || true)"
  configured="$(grep -F '[layer] configured surface' "$LOG_LAYER" | tail -n1 || true)"
  last_err="$(grep -E 'Error:|\\[layer\\].*error' "$LOG_LAYER" | tail -n1 || true)"

  if [[ -n "$selected" ]]; then
    echo "  selected_output=${selected#*: }"
  elif [[ -n "$fallback" ]]; then
    echo "  selected_output=fallback(default output)"
    echo "  detail=${fallback#*] }"
  else
    echo "  selected_output=unknown (sin traza reciente)"
  fi

  if [[ -n "$configured" ]]; then
    echo "  configured=${configured#*] }"
  fi
  if [[ -n "$last_err" ]]; then
    echo "  last_error=$last_err"
  fi
}

cmd_doctor() {
  local fix=0
  local all_instances=0
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --fix) fix=1 ;;
      --all-instances) all_instances=1 ;;
      *)
        echo "[x] Opcion desconocida para doctor: $1"
        exit 1
        ;;
    esac
    shift
  done
  local output_target
  output_target="$(cfg_get output_target mpvpaper)"
  local monitor
  monitor="$(cfg_get monitor DP-1)"
  local dynamic_color
  dynamic_color="$(cfg_get dynamic_color 0)"

  local fail=0
  echo "[doctor] Dependencias requeridas"
  local dep
  local required_deps=(cargo rustc cava mpv)
  if [[ "$output_target" == "mpvpaper" ]]; then
    required_deps+=(mpvpaper)
  fi
  for dep in "${required_deps[@]}"; do
    if command -v "$dep" >/dev/null 2>&1; then
      echo "  [ok] $dep"
    else
      echo "  [x] $dep"
      fail=1
    fi
  done

  echo "[doctor] Dependencias opcionales"
  for dep in hyprctl magick convert; do
    if command -v "$dep" >/dev/null 2>&1; then
      echo "  [ok] $dep"
    else
      echo "  [i] $dep (opcional, no encontrado)"
    fi
  done

  echo "[doctor] Contrato kitowall status"
  if [[ "$dynamic_color" == "1" ]]; then
    echo "  [i] dynamic_color=1 -> validacion requerida"
    if ! kitowall_contract_check "$monitor"; then
      fail=1
    fi
  else
    if command -v kitowall >/dev/null 2>&1 && command -v node >/dev/null 2>&1; then
      if ! kitowall_contract_check "$monitor"; then
        echo "  [!] validacion fallo (no bloqueante: dynamic_color=0)"
      fi
    else
      echo "  [i] omitido (dynamic_color=0 y kitowall/node no presentes)"
    fi
  fi

  echo "[doctor] Configuracion"
  if [[ -f "$CFG" ]]; then
    echo "  [ok] $CFG"
  else
    echo "  [x] Falta $CFG"
    fail=1
  fi
  if [[ -f "$CAVA_CFG" ]]; then
    echo "  [ok] $CAVA_CFG"
  else
    echo "  [x] Falta $CAVA_CFG"
    fail=1
  fi

  local fifo_v fifo_c
  fifo_v="$(cfg_get fifo_video /tmp/kitsune-spectrum.rgba)"
  fifo_c="$(cfg_get fifo_cava /tmp/cava-rs.raw)"

  echo "[doctor] Runtime/FIFOs"
  if [[ -p "$fifo_v" ]]; then
    echo "  [ok] FIFO video existe: $fifo_v"
  else
    echo "  [!] FIFO video no existe aun: $fifo_v"
  fi
  if [[ -p "$fifo_c" ]]; then
    echo "  [ok] FIFO cava existe: $fifo_c"
  else
    echo "  [!] FIFO cava no existe aun: $fifo_c"
  fi

  echo "[doctor] Procesos"
  local f b s
  for f in "$PID_REN" "$PID_CAVA" "$PID_MPV" "$PID_LAYER" "$PID_COLOR" "$PID_MON"; do
    b="$(basename "$f")"
    s="$(pid_state "$f")"
    case "$s" in
      running:*) echo "  [ok] $b ${s#running:}" ;;
      stale:*) echo "  [!] $b stale pid=${s#stale:}" ;;
      *) echo "  [i] $b no activo" ;;
    esac
  done

  if [[ "$fix" == "1" ]]; then
    echo "[doctor] --fix aplicado"
    mkdir -p "$RUN_DIR"
    for f in "$PID_REN" "$PID_CAVA" "$PID_MPV" "$PID_LAYER" "$PID_COLOR" "$PID_MON"; do
      s="$(pid_state "$f")"
      if [[ "$s" == stale:* || "$s" == stale ]]; then
        rm -f "$f"
        echo "  [fix] removido stale pid: $f"
      fi
    done

    if ! stack_is_running; then
      rm -f "$fifo_v" "$fifo_c"
      mkfifo "$fifo_v"
      mkfifo "$fifo_c"
      echo "  [fix] recreados FIFOs"
    else
      echo "  [fix] stack activo: no se tocaron FIFOs"
    fi

    if [[ -f "$CAVA_CFG" ]]; then
      sed -i "s|^raw_target = .*|raw_target = ${fifo_c}|" "$CAVA_CFG"
      echo "  [fix] cava.conf sincronizado con fifo_cava"
    fi
  fi

  echo "[doctor] Logs recientes"
  for f in "$LOG_RENDERER" "$LOG_CAVA" "$LOG_MPV" "$LOG_LAYER"; do
    if [[ -f "$f" ]]; then
      echo "--- $f"
      tail -n 3 "$f" || true
    fi
  done

  if [[ "$all_instances" == "1" ]]; then
    echo "[doctor] Instancias (all)"
    local any=0 id cfg_inst run_pref imon r c m l st dyn_i
    for id in $(instance_ids_all); do
      any=1
      cfg_inst="$(instance_cfg_path "$id")"
      run_pref="$(instance_run_prefix "$id")"
      imon="$(instance_monitor_from_cfg "$cfg_inst")"
      dyn_i="$(cfg_get_in_file "$cfg_inst" dynamic_color 0)"
      r="$(pid_state "$run_pref/renderer.pid")"
      c="$(pid_state "$run_pref/cava.pid")"
      m="$(pid_state "$run_pref/mpvpaper.pid")"
      l="$(pid_state "$run_pref/layer.pid")"
      st="DOWN"
      if [[ "$r" == running:* && "$c" == running:* && ( "$m" == running:* || "$l" == running:* ) ]]; then
        st="UP"
      elif [[ "$r" == running:* || "$c" == running:* || "$m" == running:* || "$l" == running:* ]]; then
        st="PARTIAL"
      fi
      echo "  [$id] state=$st monitor=${imon:-?} renderer=$r cava=$c mpvpaper=$m layer=$l"
      if [[ -n "$imon" ]]; then
        if [[ "$dyn_i" == "1" ]]; then
          if ! kitowall_contract_check "$imon"; then
            fail=1
          fi
        else
          if command -v kitowall >/dev/null 2>&1 && command -v node >/dev/null 2>&1; then
            if ! kitowall_contract_check "$imon"; then
              echo "  [!] [$id] validacion kitowall fallo (no bloqueante: dynamic_color=0)"
            fi
          else
            echo "  [i] [$id] validacion kitowall omitida (dynamic_color=0)"
          fi
        fi
      fi
    done
    if [[ "$any" -eq 0 ]]; then
      echo "  (sin instancias)"
    fi
  fi

  if [[ "$fail" == "1" ]]; then
    exit 1
  fi
}

cmd_livewallpapers() {
  local sub="${1:-status}"
  shift || true
  case "$sub" in
    status)
      local as_json=0
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --json) as_json=1 ;;
          *)
            echo "Uso: kitsune livewallpapers status [--json]"
            exit 1
            ;;
        esac
        shift
      done

      local dep
      local missing=()
      local required=(steamcmd kitowall node systemctl)
      local optional=(python3 ffmpeg)
      local node_bin
      local has_node=0
      local has_kitowall=0
      local install_cmd

      node_bin="$(resolve_node_bin || true)"
      if [[ -n "$node_bin" ]]; then
        has_node=1
      fi
      if command -v kitowall >/dev/null 2>&1 || resolve_kitowall_cli_path >/dev/null 2>&1; then
        has_kitowall=1
      fi

      for dep in "${required[@]}"; do
        case "$dep" in
          node)
            if [[ "$has_node" != "1" ]]; then
              missing+=("$dep")
            fi
            ;;
          kitowall)
            if [[ "$has_kitowall" != "1" ]]; then
              missing+=("$dep")
            fi
            ;;
          *)
            if ! command -v "$dep" >/dev/null 2>&1; then
              missing+=("$dep")
            fi
            ;;
        esac
      done

      local ok=1
      if [[ "${#missing[@]}" -gt 0 ]]; then
        ok=0
      fi
      install_cmd="$(resolve_livewallpapers_install_cmd || true)"
      if [[ -z "$install_cmd" ]]; then
        install_cmd="install steamcmd manually (pacman/yay/paru)"
      fi

      if [[ "$as_json" == "1" ]]; then
        local dep_json req opt miss
        dep_json="{"
        for dep in "${required[@]}"; do
          case "$dep" in
            node) dep_json+="\"$dep\":$([[ "$has_node" == "1" ]] && echo true || echo false)," ;;
            kitowall) dep_json+="\"$dep\":$([[ "$has_kitowall" == "1" ]] && echo true || echo false)," ;;
            *) dep_json+="\"$dep\":$(command -v "$dep" >/dev/null 2>&1 && echo true || echo false)," ;;
          esac
        done
        for dep in "${optional[@]}"; do
          if command -v "$dep" >/dev/null 2>&1; then
            dep_json+="\"$dep\":true,"
          else
            dep_json+="\"$dep\":false,"
          fi
        done
        dep_json="${dep_json%,}}"
        miss="["
        for dep in "${missing[@]}"; do
          miss+="\"$dep\","
        done
        miss="${miss%,}]"
        req="[\"${required[*]}\"]"
        req="${req// /\",\"}"
        opt="[\"${optional[*]}\"]"
        opt="${opt// /\",\"}"
        echo "{\"ok\":$([[ "$ok" == "1" ]] && echo true || echo false),\"required\":$req,\"optional\":$opt,\"deps\":$dep_json,\"missing\":$miss,\"install\":\"$install_cmd\"}"
        return
      fi

      echo "[livewallpapers] Dependencias requeridas"
      for dep in "${required[@]}"; do
        case "$dep" in
          node)
            if [[ "$has_node" == "1" ]]; then
              echo "  [ok] $dep ($node_bin)"
            else
              echo "  [x] $dep"
            fi
            ;;
          kitowall)
            if [[ "$has_kitowall" == "1" ]]; then
              if command -v kitowall >/dev/null 2>&1; then
                echo "  [ok] $dep ($(command -v kitowall))"
              else
                echo "  [ok] $dep ($(resolve_kitowall_cli_path || true))"
              fi
            else
              echo "  [x] $dep"
            fi
            ;;
          *)
            if command -v "$dep" >/dev/null 2>&1; then
              echo "  [ok] $dep"
            else
              echo "  [x] $dep"
            fi
            ;;
        esac
      done
      echo "[livewallpapers] Dependencias opcionales"
      for dep in "${optional[@]}"; do
        if command -v "$dep" >/dev/null 2>&1; then
          echo "  [ok] $dep"
        else
          echo "  [i] $dep (opcional)"
        fi
      done
      if [[ "$ok" == "1" ]]; then
        echo "[ok] Entorno listo para Workshop + SteamCMD"
      else
        echo "[x] Faltan dependencias: ${missing[*]}"
        echo "[i] Instalar: $install_cmd"
        exit 1
      fi
      ;;
    install)
      local print_only=0
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --print) print_only=1 ;;
          *)
            echo "Uso: kitsune livewallpapers install [--print]"
            exit 1
            ;;
        esac
        shift
      done
      local cmd_install
      cmd_install="$(resolve_livewallpapers_install_cmd || true)"
      if [[ -z "$cmd_install" ]]; then
        echo "[x] No se detecto gestor compatible automaticamente (pacman/yay/paru)"
        echo "[i] Instala steamcmd manualmente con tu gestor de paquetes."
        exit 1
      fi
      if [[ "$print_only" == "1" ]]; then
        echo "$cmd_install"
        return
      fi
      if [[ ! -t 0 || ! -t 1 ]]; then
        echo "[x] Instalacion interactiva no disponible sin terminal (TTY)."
        echo "[i] Ejecuta manualmente en tu terminal:"
        echo "    $cmd_install"
        exit 1
      fi
      echo "[i] Ejecutando: $cmd_install"
      bash -lc "$cmd_install"
      ;;
    *)
      echo "Uso: kitsune livewallpapers <status|install> ..."
      exit 1
      ;;
  esac
}

cmd_config() {
  local sub="${1:-}"
  shift || true

  case "$sub" in
    get)
      local key="${1:-}"
      if [[ -z "$key" ]]; then
        echo "Uso: kitsune config get <key>"
        exit 1
      fi
      if cfg_has_key "$key"; then
        cfg_get "$key" ""
      else
        echo "[x] Key no encontrada: $key"
        exit 1
      fi
      ;;
    set)
      local key="${1:-}"
      local val="${2:-}"
      if [[ -z "$key" || -z "$val" ]]; then
        echo "Uso: kitsune config set <key> <value> [--apply|--restart]"
        exit 1
      fi
      shift 2 || true
      local apply=0
      local restart=0
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --apply) apply=1 ;;
          --restart) restart=1 ;;
          *)
            echo "[x] Opcion desconocida: $1"
            exit 1
            ;;
        esac
        shift
      done
      cfg_set "$key" "$val"
      echo "[OK] $key=$val"
      if [[ "$restart" == "1" || "$apply" == "1" ]]; then
        ./scripts/kitsune.sh restart
      fi
      ;;
    list)
      local effective=0
      if [[ "${1:-}" == "--effective" ]]; then
        effective=1
      fi
      awk -F'=' '/^[[:space:]]*#/ {next} /^[[:space:]]*$/ {next} {k=$1; v=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", k); gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); print k "=" v}' "$CFG"
      if [[ "$effective" == "1" ]]; then
        local mon mon_real mon_reason out_target test_profile
        mon="$(cfg_get monitor DP-1)"
        mon_real="$(cat "$RUN_DIR/target_monitor" 2>/dev/null || true)"
        mon_reason="$(cat "$RUN_DIR/target_reason" 2>/dev/null || true)"
        out_target="$(cfg_get output_target mpvpaper)"
        test_profile="$(cfg_get test_profile_file ./config/profiles/test.profile)"
        echo "stack_running=$(stack_is_running && echo 1 || echo 0)"
        echo "monitor_selected=$mon"
        echo "monitor_real=${mon_real:-n/a}"
        echo "monitor_reason=${mon_reason:-n/a}"
        echo "output_target=$out_target"
        echo "fifo_video=$(cfg_get fifo_video /tmp/kitsune-spectrum.rgba)"
        echo "fifo_cava=$(cfg_get fifo_cava /tmp/cava-rs.raw)"
        echo "pid_renderer=$(pid_state "$PID_REN")"
        echo "pid_cava=$(pid_state "$PID_CAVA")"
        echo "pid_mpvpaper=$(pid_state "$PID_MPV")"
        echo "pid_layer=$(pid_state "$PID_LAYER")"
        echo "log_renderer=$LOG_RENDERER"
        echo "log_cava=$LOG_CAVA"
        echo "log_mpvpaper=$LOG_MPV"
        echo "log_layer=$LOG_LAYER"
        echo "test_profile_file=$test_profile"
      fi
      ;;
    *)
      echo "Uso: kitsune config <get|set|list> ..."
      exit 1
      ;;
  esac
}

cmd_profiles() {
  local sub="${1:-}"
  shift || true
  case "$sub" in
    list)
      local kind="${1:-all}"
      local f name
      shopt -s nullglob
      for f in ./config/profiles/*.profile; do
        name="$(basename "$f" .profile)"
        case "$kind" in
          all) echo "$name" ;;
          bars)
            if [[ "$name" == bars* ]]; then
              echo "$name"
            fi
            ;;
          ring)
            if [[ "$name" == ring* ]]; then
              echo "$name"
            fi
            ;;
          *)
            echo "[x] Filtro invalido: $kind"
            exit 1
            ;;
        esac
      done
      ;;
    show)
      local name="${1:-}"
      if [[ -z "$name" ]]; then
        echo "Uso: kitsune profiles show <name>"
        exit 1
      fi
      local p="./config/profiles/${name}.profile"
      [[ -f "$p" ]] || p="$name"
      if [[ ! -f "$p" ]]; then
        echo "[x] Perfil no encontrado: $name"
        exit 1
      fi
      cat "$p"
      ;;
    set-list)
      local kind="${1:-}"
      local list="${2:-}"
      if [[ -z "$kind" || -z "$list" ]]; then
        echo "Uso: kitsune profiles set-list <bars|ring> <p1,p2,...>"
        exit 1
      fi
      if [[ "$kind" != "bars" && "$kind" != "ring" ]]; then
        echo "[x] kind invalido: $kind"
        exit 1
      fi
      local key
      key="${kind}_profiles"
      local IFS=','
      local p clean=""
      read -r -a arr <<< "$list"
      for p in "${arr[@]}"; do
        p="$(printf '%s' "$p" | xargs)"
        [[ -z "$p" ]] && continue
        if [[ ! -f "./config/profiles/${p}.profile" ]]; then
          echo "[x] Perfil no existe: $p"
          exit 1
        fi
        if [[ -z "$clean" ]]; then
          clean="$p"
        else
          clean="${clean},${p}"
        fi
      done
      if [[ -z "$clean" ]]; then
        echo "[x] lista vacia"
        exit 1
      fi
      cfg_set "$key" "$clean"
      echo "[OK] $key=$clean"
      ;;
    set-static)
      local name="${1:-}"
      if [[ -z "$name" ]]; then
        echo "Uso: kitsune profiles set-static <name>"
        exit 1
      fi
      if [[ ! -f "./config/profiles/${name}.profile" ]]; then
        echo "[x] Perfil no existe: $name"
        exit 1
      fi
      cfg_set static_profile "$name"
      echo "[OK] static_profile=$name"
      ;;
    rotate)
      local v="${1:-}"
      case "$v" in
        on|1|true|yes) ./scripts/set-rotate-profiles.sh 1 ;;
        off|0|false|no) ./scripts/set-rotate-profiles.sh 0 ;;
        *)
          echo "Uso: kitsune profiles rotate <on|off>"
          exit 1
          ;;
      esac
      ;;
    clone)
      local src="${1:-}"
      local dst="${2:-}"
      if [[ -z "$src" || -z "$dst" ]]; then
        echo "Uso: kitsune profiles clone <base> <new>"
        exit 1
      fi
      local src_path="./config/profiles/${src}.profile"
      local dst_path="./config/profiles/${dst}.profile"
      if [[ ! -f "$src_path" ]]; then
        echo "[x] Perfil base no existe: $src"
        exit 1
      fi
      if [[ -f "$dst_path" ]]; then
        echo "[x] Perfil destino ya existe: $dst"
        exit 1
      fi
      cp "$src_path" "$dst_path"
      echo "[OK] clonado: $src -> $dst"
      ;;
    set)
      local name="${1:-}"
      local key="${2:-}"
      local val="${3:-}"
      if [[ -z "$name" || -z "$key" || -z "$val" ]]; then
        echo "Uso: kitsune profiles set <name> <key> <value>"
        exit 1
      fi
      local p="./config/profiles/${name}.profile"
      if [[ ! -f "$p" ]]; then
        echo "[x] Perfil no existe: $name"
        exit 1
      fi
      if grep -qE "^[[:space:]]*${key}[[:space:]]*=" "$p"; then
        sed -i "s|^[[:space:]]*${key}[[:space:]]*=.*|${key}=${val}|" "$p"
      else
        printf '%s=%s\n' "$key" "$val" >> "$p"
      fi
      echo "[OK] ${name}.profile: ${key}=${val}"
      ;;
    *)
      echo "Uso: kitsune profiles <list|show|set-list|set-static|rotate|clone|set> ..."
      exit 1
      ;;
  esac
}

resolve_group_file() {
  local file="${1:-}"
  if [[ -z "$file" ]]; then
    file="$(cfg_get group_file ./config/groups/default.group)"
  fi
  if [[ ! -f "$file" && -f "./$file" ]]; then
    file="./$file"
  fi
  if [[ ! -f "$file" && -f "./config/groups/$file" ]]; then
    file="./config/groups/$file"
  fi
  if [[ ! -f "$file" ]]; then
    return 1
  fi
  printf '%s\n' "$file"
}

normalize_layer_line() {
  local raw="$1"
  if [[ "$raw" == layer=* ]]; then
    printf '%s\n' "$raw"
  else
    printf 'layer=%s\n' "$raw"
  fi
}

cmd_group_validate() {
  local file
  file="$(resolve_group_file "${1:-}")" || {
    echo "[x] No existe: $file (ni en ./config/groups)"
    exit 1
  }

  local errors=0
  local ln=0
  declare -A seen_ids=()
  while IFS= read -r line; do
    ln=$((ln + 1))
    [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
    [[ "$line" =~ ^layer= ]] || continue

    local raw="${line#layer=}"
    IFS=',' read -r -a parts <<< "$raw"
    local enabled="${parts[0]:-}"
    local mode="${parts[1]:-}"
    local style="${parts[2]:-}"
    local profile="${parts[3]:-}"
    local color="${parts[4]:-}"
    local alpha="${parts[5]:-}"
    local runtime="${parts[6]:-}"
    local rotate="${parts[7]:-}"
    local profiles_pipe="${parts[8]:-}"

    if [[ -z "$enabled" || -z "$mode" || -z "$style" || -z "$profile" || -z "$color" || -z "$alpha" ]]; then
      echo "[x] linea $ln: faltan campos minimos"
      errors=$((errors + 1))
      continue
    fi

    if [[ "$mode" != "bars" && "$mode" != "ring" ]]; then
      echo "[x] linea $ln: mode invalido '$mode'"
      errors=$((errors + 1))
    fi

    case "$style" in
      bars|bars_fill|waves|waves_fill|dots) ;;
      *)
        echo "[x] linea $ln: style invalido '$style'"
        errors=$((errors + 1))
        ;;
    esac

    if [[ ! "$color" =~ ^#[0-9A-Fa-f]{6}$ ]]; then
      echo "[x] linea $ln: color invalido '$color' (esperado #RRGGBB)"
      errors=$((errors + 1))
    fi

    if ! awk -v a="$alpha" 'BEGIN{exit !(a ~ /^-?[0-9]+([.][0-9]+)?$/ && a>=0 && a<=1)}'; then
      echo "[x] linea $ln: alpha fuera de rango '$alpha' (esperado 0..1)"
      errors=$((errors + 1))
    fi

    if [[ ! -f "./config/profiles/${profile}.profile" ]]; then
      echo "[x] linea $ln: perfil no existe './config/profiles/${profile}.profile'"
      errors=$((errors + 1))
    fi

    if [[ "$rotate" == "1" && -z "$profiles_pipe" ]]; then
      echo "[x] linea $ln: rotate=1 requiere profiles_pipe no vacio"
      errors=$((errors + 1))
    fi

    local token lid
    for token in "${parts[@]}"; do
      if [[ "$token" == id=* ]]; then
        lid="${token#id=}"
        if [[ -z "$lid" ]]; then
          echo "[x] linea $ln: id vacio"
          errors=$((errors + 1))
        elif [[ -n "${seen_ids[$lid]:-}" ]]; then
          echo "[x] linea $ln: id duplicado '$lid' (ya usado en linea ${seen_ids[$lid]})"
          errors=$((errors + 1))
        else
          seen_ids["$lid"]="$ln"
        fi
      fi
    done
  done < "$file"

  if [[ "$errors" -eq 0 ]]; then
    echo "[OK] group valido: $file"
  else
    echo "[x] group invalido: $errors error(es)"
    exit 1
  fi
}

cmd_group_files() {
  local dir="./config/groups"
  if [[ ! -d "$dir" ]]; then
    echo "[x] No existe directorio de grupos: $dir"
    exit 1
  fi
  local f
  shopt -s nullglob
  for f in "$dir"/*.group; do
    basename "$f"
  done | sort
}

cmd_group_create() {
  local name="${1:-}"
  if [[ -z "$name" ]]; then
    echo "Uso: kitsune group create <name|name.group>"
    exit 1
  fi

  if [[ "$name" == */* ]]; then
    echo "[x] Solo nombre de archivo, sin rutas: $name"
    exit 1
  fi

  local file="$name"
  if [[ "$file" != *.group ]]; then
    file="${file}.group"
  fi
  local path="./config/groups/${file}"
  mkdir -p "./config/groups"
  if [[ -f "$path" ]]; then
    echo "[x] Group ya existe: $file"
    exit 1
  fi

  cat > "$path" <<'EOF'
# kitsune group file
# formato:
# layer=enabled,mode,style,profile,color,alpha[,runtime][,rotate][,profiles_pipe]
EOF
  echo "[OK] group creado: $file"
}

cmd_group_list_layers() {
  local file
  file="$(resolve_group_file "${1:-}")" || {
    echo "[x] No existe group file"
    exit 1
  }
  awk '
    /^[[:space:]]*#/ {next}
    /^[[:space:]]*$/ {next}
    /^layer=/ {idx++; print idx": "$0}
  ' "$file"
}

cmd_group_add_layer() {
  local spec="${1:-}"
  local file="${2:-}"
  if [[ -z "$spec" ]]; then
    echo "Uso: kitsune group add-layer \"<csv|layer=csv>\" [file.group]"
    exit 1
  fi
  file="$(resolve_group_file "$file")" || {
    echo "[x] No existe group file"
    exit 1
  }
  local line
  line="$(normalize_layer_line "$spec")"
  printf '%s\n' "$line" >> "$file"
  echo "[OK] layer agregada en $file"
}

cmd_group_update_layer() {
  local index="${1:-}"
  local spec="${2:-}"
  local file="${3:-}"
  if [[ -z "$index" || -z "$spec" ]]; then
    echo "Uso: kitsune group update-layer <index> \"<csv|layer=csv>\" [file.group]"
    exit 1
  fi
  if ! [[ "$index" =~ ^[0-9]+$ ]] || [[ "$index" -le 0 ]]; then
    echo "[x] index invalido: $index"
    exit 1
  fi
  file="$(resolve_group_file "$file")" || {
    echo "[x] No existe group file"
    exit 1
  }
  local line tmp
  line="$(normalize_layer_line "$spec")"
  tmp="$(mktemp)"
  awk -v idx="$index" -v rep="$line" '
    BEGIN {k=0; ok=0}
    {
      if ($0 ~ /^layer=/) {
        k++
        if (k==idx) {
          print rep
          ok=1
          next
        }
      }
      print $0
    }
    END {
      if (!ok) exit 42
    }
  ' "$file" > "$tmp" || {
    rm -f "$tmp"
    echo "[x] index fuera de rango: $index"
    exit 1
  }
  mv "$tmp" "$file"
  echo "[OK] layer #$index actualizada en $file"
}

cmd_group_remove_layer() {
  local index="${1:-}"
  local file="${2:-}"
  if [[ -z "$index" ]]; then
    echo "Uso: kitsune group remove-layer <index> [file.group]"
    exit 1
  fi
  if ! [[ "$index" =~ ^[0-9]+$ ]] || [[ "$index" -le 0 ]]; then
    echo "[x] index invalido: $index"
    exit 1
  fi
  file="$(resolve_group_file "$file")" || {
    echo "[x] No existe group file"
    exit 1
  }
  local tmp
  tmp="$(mktemp)"
  awk -v idx="$index" '
    BEGIN {k=0; ok=0}
    {
      if ($0 ~ /^layer=/) {
        k++
        if (k==idx) { ok=1; next }
      }
      print $0
    }
    END {
      if (!ok) exit 42
    }
  ' "$file" > "$tmp" || {
    rm -f "$tmp"
    echo "[x] index fuera de rango: $index"
    exit 1
  }
  mv "$tmp" "$file"
  echo "[OK] layer #$index eliminada de $file"
}

cmd_monitors() {
  local sub="${1:-}"
  shift || true
  case "$sub" in
    list)
      if command -v hyprctl >/dev/null 2>&1; then
        local mon_dump
        mon_dump="$(hyprctl monitors 2>/dev/null || true)"
        if [[ -z "$mon_dump" ]]; then
          echo "hyprctl disponible pero sin salida de monitores; monitor config: $(cfg_get monitor DP-1)"
          return 0
        fi
        printf '%s\n' "$mon_dump" | awk '
          /^Monitor / {mon=$2; focused="no"; w="?"; h="?"}
          /^[[:space:]]*focused:[[:space:]]*yes/ {focused="yes"}
          /^[[:space:]]*[0-9]+x[0-9]+@/ {
            split($1, a, "x");
            w=a[1];
            split(a[2], b, "@");
            h=b[1]
          }
          /^$/ {if (mon!="") print mon " focused=" focused " size=" w "x" h; mon=""}
          END {if (mon!="") print mon " focused=" focused " size=" w "x" h}
        '
      else
        echo "hyprctl no disponible. monitor actual en config: $(cfg_get monitor DP-1)"
      fi
      ;;
    set)
      local mon="${1:-}"
      if [[ -z "$mon" ]]; then
        echo "Uso: kitsune monitor set <name>"
        exit 1
      fi
      if command -v hyprctl >/dev/null 2>&1; then
        if ! hyprctl monitors 2>/dev/null | awk '/^Monitor /{print $2}' | grep -Fxq "$mon"; then
          echo "[x] monitor no detectado por hyprctl: $mon"
          exit 1
        fi
      fi
      cfg_set monitor "$mon"
      echo "[OK] monitor=$mon"
      ;;
    *)
      echo "Uso: kitsune monitors list | kitsune monitor set <name>"
      exit 1
      ;;
  esac
}

pick_profile_from_mode() {
  local direction="$1"
  local mode list_key list_csv current idx next_idx
  mode="$(cfg_get mode bars)"
  if [[ "$mode" == "ring" ]]; then
    list_key="ring_profiles"
  else
    list_key="bars_profiles"
  fi
  list_csv="$(cfg_get "$list_key" "")"
  current="$(cfg_get static_profile "")"

  if [[ -z "$list_csv" ]]; then
    echo ""
    return
  fi

  IFS=',' read -r -a arr <<< "$list_csv"
  local n="${#arr[@]}"
  if [[ "$n" -eq 0 ]]; then
    echo ""
    return
  fi

  idx=0
  local i
  for i in "${!arr[@]}"; do
    if [[ "${arr[$i]}" == "$current" ]]; then
      idx="$i"
      break
    fi
  done

  case "$direction" in
    next) next_idx=$(( (idx + 1) % n )) ;;
    prev) next_idx=$(( (idx - 1 + n) % n )) ;;
    shuffle)
      if [[ -f "$SEED_FILE" ]]; then
        local seed
        seed="$(cat "$SEED_FILE" 2>/dev/null || true)"
        if [[ "$seed" =~ ^[0-9]+$ ]]; then
          RANDOM="$seed"
        fi
      fi
      next_idx=$(( RANDOM % n ))
      ;;
    *) next_idx="$idx" ;;
  esac

  echo "${arr[$next_idx]}"
}

cmd_rotate() {
  local sub="${1:-}"
  shift || true

  case "$sub" in
    0|1)
      ./scripts/set-rotate-profiles.sh "$sub"
      ;;
    next|prev|shuffle)
      local apply=0
      if [[ "${1:-}" == "--apply" ]]; then
        apply=1
      fi
      local p
      p="$(pick_profile_from_mode "$sub")"
      if [[ -z "$p" ]]; then
        echo "[x] No hay perfiles en lista para el modo actual"
        exit 1
      fi
      cfg_set static_profile "$p"
      cfg_set rotate_profiles 0
      echo "[OK] static_profile=$p (rotate_profiles=0 para preview manual)"
      if [[ "$apply" == "1" ]]; then
        ./scripts/kitsune.sh restart
      fi
      ;;
    seed)
      local n="${1:-}"
      if [[ ! "$n" =~ ^[0-9]+$ ]]; then
        echo "Uso: kitsune rotate seed <n>"
        exit 1
      fi
      mkdir -p "$RUN_DIR"
      echo "$n" > "$SEED_FILE"
      echo "[OK] rotate seed=$n"
      ;;
    *)
      echo "Uso: kitsune rotate <0|1|next|prev|shuffle|seed> ..."
      exit 1
      ;;
  esac
}

cmd_autostart() {
  local action="${1:-}"
  shift || true
  local monitor=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --monitor)
        monitor="${2:-}"
        shift
        ;;
      *)
        echo "[x] Opcion desconocida para autostart: $1"
        exit 1
        ;;
    esac
    shift
  done

  local svc_dir="$HOME/.config/systemd/user"
  local svc_file="$svc_dir/kitsune.service"
  local svc_tpl="$svc_dir/kitsune@.service"
  local monitor_unit=""
  if [[ -n "$monitor" ]]; then
    monitor_unit="kitsune@${monitor}.service"
  fi

  case "$action" in
    enable)
      mkdir -p "$svc_dir"
      if [[ -n "$monitor" ]]; then
        cat > "$svc_tpl" <<EOF_SVC
[Unit]
Description=Kitsune Visualizer Stack (%i)
After=graphical-session.target

[Service]
Type=simple
WorkingDirectory=$(pwd)
ExecStart=/bin/bash -lc 'cd "$(pwd)" && ./scripts/kitsune.sh start "%i"'
ExecStop=/bin/bash -lc 'cd "$(pwd)" && ./scripts/kitsune.sh stop "%i"'
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
EOF_SVC
        echo "[OK] template escrito: $svc_tpl"
      else
        cat > "$svc_file" <<EOF_SVC
[Unit]
Description=Kitsune Visualizer Stack (global)
After=graphical-session.target

[Service]
Type=simple
WorkingDirectory=$(pwd)
ExecStart=$(pwd)/scripts/start.sh
ExecStop=$(pwd)/scripts/stop.sh
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
EOF_SVC
        echo "[OK] service escrito: $svc_file"
      fi
      if command -v systemctl >/dev/null 2>&1; then
        systemctl --user daemon-reload || true
        if [[ -n "$monitor" ]]; then
          systemctl --user enable --now "$monitor_unit" || true
          systemctl --user status "$monitor_unit" --no-pager || true
        else
          systemctl --user enable --now kitsune.service || true
          systemctl --user status kitsune.service --no-pager || true
        fi
      else
        echo "[i] systemctl no disponible; habilita manualmente el servicio"
      fi
      ;;
    disable)
      if command -v systemctl >/dev/null 2>&1; then
        if [[ -n "$monitor" ]]; then
          systemctl --user disable --now "$monitor_unit" || true
        else
          systemctl --user disable --now kitsune.service || true
        fi
        systemctl --user daemon-reload || true
      fi
      if [[ -n "$monitor" ]]; then
        echo "[OK] autostart deshabilitado para monitor=$monitor"
      else
        rm -f "$svc_file"
        echo "[OK] autostart global deshabilitado"
      fi
      ;;
    status)
      if command -v systemctl >/dev/null 2>&1; then
        if [[ -n "$monitor" ]]; then
          systemctl --user status "$monitor_unit" --no-pager || true
        else
          systemctl --user status kitsune.service --no-pager || true
        fi
      else
        if [[ -n "$monitor" ]]; then
          echo "[i] systemctl no disponible (monitor=$monitor)"
        elif [[ -f "$svc_file" ]]; then
          echo "service presente: $svc_file"
        else
          echo "service no instalado"
        fi
      fi
      ;;
    list)
      echo "Autostart units:"
      if [[ -f "$svc_file" ]]; then
        echo "  - kitsune.service (global)"
      fi
      if [[ -f "$svc_tpl" ]]; then
        echo "  - kitsune@.service (template)"
      fi
      if command -v systemctl >/dev/null 2>&1; then
        systemctl --user list-unit-files 'kitsune*.service' --no-legend 2>/dev/null || true
      fi
      ;;
    *)
      echo "Uso: kitsune autostart <enable|disable|status|list> [--monitor <name>]"
      exit 1
      ;;
  esac
}

cmd_clean() {
  local force=0
  if [[ "${1:-}" == "--force" ]]; then
    force=1
  fi

  if [[ "$force" == "1" ]]; then
    ./scripts/stop.sh || true
  fi

  local fifo_v fifo_c
  fifo_v="$(cfg_get fifo_video /tmp/kitsune-spectrum.rgba)"
  fifo_c="$(cfg_get fifo_cava /tmp/cava-rs.raw)"

  rm -f "$PID_REN" "$PID_CAVA" "$PID_MPV" "$PID_LAYER" "$PID_COLOR" "$PID_MON" "$SEED_FILE"
  rm -f "$fifo_v" "$fifo_c"
  rm -f "$LOG_RENDERER" "$LOG_CAVA" "$LOG_MPV" "$LOG_LAYER" "$LOG_COLOR" "$LOG_MON"

  echo "[OK] runtime limpio"
}

ensure_default_files() {
  if [[ ! -f "$DEFAULT_CFG" ]]; then
    cp "$CFG" "$DEFAULT_CFG"
  fi
  if [[ ! -f "$DEFAULT_CAVA_CFG" ]]; then
    cp "$CAVA_CFG" "$DEFAULT_CAVA_CFG"
  fi
}

cmd_reset() {
  local restart=0
  if [[ "${1:-}" == "--restart" ]]; then
    restart=1
  fi

  ensure_default_files

  local ts backup_dir
  ts="$(date +%Y%m%d-%H%M%S)"
  backup_dir="./config/backups/$ts"
  mkdir -p "$backup_dir"
  cp "$CFG" "$backup_dir/base.conf.bak"
  cp "$CAVA_CFG" "$backup_dir/cava.conf.bak"

  cp "$DEFAULT_CFG" "$CFG"
  cp "$DEFAULT_CAVA_CFG" "$CAVA_CFG"

  echo "[OK] reset aplicado"
  echo "  backup: $backup_dir"

  if [[ "$restart" == "1" ]]; then
    ./scripts/kitsune.sh restart
  fi
}

cmd_benchmark() {
  local seconds="${1:-10}"
  if ! [[ "$seconds" =~ ^[0-9]+$ ]] || [[ "$seconds" -le 0 ]]; then
    echo "Uso: kitsune benchmark [seconds]"
    exit 1
  fi

  local state
  state="$(pid_state "$PID_REN")"
  if [[ "$state" != running:* ]]; then
    echo "[x] renderer no esta corriendo; inicia con: kitsune start"
    exit 1
  fi
  local pid="${state#running:}"

  local i cpu rss cpu_sum=0 rss_sum=0 samples=0
  echo "[i] benchmark ${seconds}s (pid=$pid)..."
  for ((i=0; i<seconds; i++)); do
    cpu="$(ps -p "$pid" -o %cpu= 2>/dev/null | awk '{print $1+0}' || echo 0)"
    rss="$(ps -p "$pid" -o rss= 2>/dev/null | awk '{print $1+0}' || echo 0)"
    cpu_sum="$(awk -v a="$cpu_sum" -v b="$cpu" 'BEGIN{printf "%.2f", a+b}')"
    rss_sum=$((rss_sum + rss))
    samples=$((samples + 1))
    sleep 1
  done

  local cpu_avg rss_avg
  cpu_avg="$(awk -v s="$cpu_sum" -v n="$samples" 'BEGIN{if(n>0) printf "%.2f", s/n; else print "0.00"}')"
  rss_avg=$(( samples > 0 ? rss_sum / samples : 0 ))

  echo "Benchmark result:"
  echo "  seconds=$seconds"
  echo "  backend=$(cfg_get backend cpu)"
  echo "  target_fps=$(cfg_get fps 60)"
  echo "  avg_cpu_percent=$cpu_avg"
  echo "  avg_rss_kb=$rss_avg"

  local fps
  fps="$(grep -Eo 'fps[=: ]+[0-9]+(\.[0-9]+)?' "$LOG_RENDERER" 2>/dev/null | tail -n1 | grep -Eo '[0-9]+(\.[0-9]+)?' || true)"
  if [[ -n "$fps" ]]; then
    echo "  fps_real=$fps"
  else
    echo "  fps_real=n/a"
  fi
}

cmd_debug() {
  local sub="${1:-}"
  shift || true
  case "$sub" in
    overlay)
      local val="${1:-}"
      shift || true
      if [[ "$val" != "0" && "$val" != "1" ]]; then
        echo "Uso: kitsune debug overlay <0|1> [--apply]"
        exit 1
      fi
      cfg_set debug_overlay "$val"
      echo "[OK] debug_overlay=$val"
      echo "[i] Nota: requiere soporte del renderer para mostrarse en pantalla"
      if [[ "${1:-}" == "--apply" ]]; then
        ./scripts/kitsune.sh restart
      fi
      ;;
    *)
      echo "Uso: kitsune debug overlay <0|1> [--apply]"
      exit 1
      ;;
  esac
}

cmd_restart() {
  local rebuild="${1:-}"
  if [[ "$rebuild" == "--rebuild" || -z "$rebuild" ]]; then
    ./scripts/stop.sh || true
    ./scripts/start.sh
  else
    echo "Uso: kitsune restart [--rebuild]"
    exit 1
  fi
}

cmd="${1:-help}"
if [[ $# -gt 0 ]]; then
  shift
fi

case "$cmd" in
  help|-h|--help)
    help_cmd "${1:-}"
    ;;
  install)
    ./scripts/install.sh "$@"
    ;;
  start)
    cmd_start "$@"
    ;;
  stop)
    cmd_stop "$@"
    ;;
  restart)
    cmd_restart "$@"
    ;;
  status)
    cmd_status "$@"
    ;;
  doctor)
    cmd_doctor "$@"
    ;;
  logs)
    cmd_logs "$@"
    ;;
  layer-status)
    cmd_layer_status
    ;;
  run)
    if [[ "${1:-}" == "--config" ]]; then
      if [[ -z "${2:-}" ]]; then
        echo "[x] Uso: kitsune run [--config <path>]"
        exit 1
      fi
      ./target/release/kitsune --config "$2"
    else
      ./target/release/kitsune --config ./config/base.conf
    fi
    ;;
  config)
    cmd_config "$@"
    ;;
  visual)
    ./scripts/set-visual.sh "$@"
    ;;
  style)
    ./scripts/set-style.sh "$@"
    ;;
  mode)
    ./scripts/set-mode.sh "$@"
    ;;
  wave-roundness)
    ./scripts/set-wave-roundness.sh "$@"
    ;;
  ring-fill-softness)
    ./scripts/set-ring-fill-softness.sh "$@"
    ;;
  waves-fill-preset)
    ./scripts/set-waves-fill-preset.sh "$@"
    ;;
  backend)
    ./scripts/set-backend.sh "$@"
    ;;
  output-target)
    ./scripts/set-output-target.sh "$@"
    ;;
  spectrum-mode)
    ./scripts/set-spectrum-mode.sh "$@"
    ;;
  group-file)
    ./scripts/set-group-file.sh "$@"
    ;;
  group)
    case "${1:-}" in
      files)
        shift
        cmd_group_files "$@"
        ;;
      create)
        shift || true
        cmd_group_create "$@"
        ;;
      validate)
        shift || true
        cmd_group_validate "$@"
        ;;
      list-layers)
        shift
        cmd_group_list_layers "$@"
        ;;
      add-layer)
        shift
        cmd_group_add_layer "$@"
        ;;
      update-layer)
        shift
        cmd_group_update_layer "$@"
        ;;
      remove-layer)
        shift
        cmd_group_remove_layer "$@"
        ;;
      *)
        echo "Uso: kitsune group <files|create|validate|list-layers|add-layer|update-layer|remove-layer> ..."
        exit 1
        ;;
    esac
    ;;
  runtime)
    ./scripts/set-runtime-mode.sh "$@"
    ;;
  rotate)
    cmd_rotate "$@"
    ;;
  rotation)
    ./scripts/set-rotation.sh "$@"
    ;;
  profiles)
    cmd_profiles "$@"
    ;;
  test-load)
    ./scripts/test-profile-load.sh "$@"
    ;;
  profile-edit)
    ./scripts/profile-edit.sh "$@"
    ;;
  tune)
    ./scripts/tune.sh "$@"
    ;;
  dynamic-color)
    ./scripts/set-dynamic-color.sh "$@"
    ;;
  color-poll)
    ./scripts/set-color-poll.sh "$@"
    ;;
  colorwatch)
    mon="${1:-DP-1}"
    out="${2:-/tmp/kitsune-accent.hex}"
    intv="${3:-2}"
    once="${4:-}"
    if [[ "$once" == "--once" || "$once" == "once" ]]; then
      ./scripts/wallpaper-accent-watcher.sh "$mon" "$out" "$intv" --once
    else
      ./scripts/wallpaper-accent-watcher.sh "$mon" "$out" "$intv"
    fi
    ;;
  postfx)
    ./scripts/set-postfx.sh "$@"
    ;;
  particles)
    ./scripts/set-particles.sh "$@"
    ;;
  particles-look)
    ./scripts/set-particles-look.sh "$@"
    ;;
  particles-preset)
    ./scripts/set-particles-preset.sh "$@"
    ;;
  monitors)
    cmd_monitors "$@"
    ;;
  monitor)
    if [[ "${1:-}" == "set" ]]; then
      shift
    fi
    cmd_monitors set "$@"
    ;;
  monitor-fallback)
    ./scripts/set-monitor-fallback.sh "$@"
    ;;
  autostart)
    cmd_autostart "$@"
    ;;
  instances)
    cmd_instances "$@"
    ;;
  instance-status)
    cmd_instances status "$@"
    ;;
  livewallpapers)
    cmd_livewallpapers "$@"
    ;;
  clean)
    cmd_clean "$@"
    ;;
  reset)
    cmd_reset "$@"
    ;;
  benchmark)
    cmd_benchmark "$@"
    ;;
  debug)
    cmd_debug "$@"
    ;;
  *)
    echo "[x] Comando desconocido: $cmd"
    echo
    usage
    exit 1
    ;;
esac
