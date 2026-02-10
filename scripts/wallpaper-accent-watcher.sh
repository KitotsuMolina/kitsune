#!/usr/bin/env bash
set -euo pipefail

MONITOR="${1:-DP-1}"
OUT_FILE="${2:-/tmp/kitsune-accent.hex}"
INTERVAL="${3:-2}"
MODE="${4:-watch}"

if ! [[ "$INTERVAL" =~ ^[0-9]+$ ]] || [[ "$INTERVAL" -lt 1 ]]; then
  echo "Uso: $0 <monitor> <out_file> <intervalo_segundos>=1.."
  exit 1
fi

last_wall=""
last_color=""
last_warn_ts=0

extract_wallpaper_path() {
  local mon="$1"
  local p=""

  # 0) KitoWall CLI status (fuente principal en este setup)
  if [[ -z "$p" ]] && command -v kitowall >/dev/null 2>&1 && command -v node >/dev/null 2>&1; then
    p="$(timeout 3 kitowall status 2>/dev/null \
      | node -e '
        let s = "";
        process.stdin.on("data", d => s += d);
        process.stdin.on("end", () => {
          try {
            const j = JSON.parse(s || "{}");
            const mon = process.argv[1];
            const last = j.last_set || {};
            if (typeof last[mon] === "string") return process.stdout.write(last[mon]);
            const alt = Object.keys(last).find(k => k.toLowerCase() === mon.toLowerCase());
            if (alt && typeof last[alt] === "string") return process.stdout.write(last[alt]);
          } catch (_) {}
        });
      ' "$mon" || true)"
  fi

  # 1) swww (si esta activo)
  if [[ -z "$p" ]] && command -v swww >/dev/null 2>&1; then
    p="$(swww query 2>/dev/null | awk -v m="$mon" '
      BEGIN{f=0}
      /^Output:/ {f = ($2 == m)}
      f && /image:/ {
        if (match($0, /image:[[:space:]]*(.*)$/, a)) {print a[1]; exit}
      }
    ' || true)"
  fi

  # 2) hyprctl hyprpaper listactive (versiones que lo soportan)
  if [[ -z "$p" ]] && command -v hyprctl >/dev/null 2>&1; then
    p="$(hyprctl hyprpaper listactive 2>/dev/null | awk -v m="$mon" '
      $0 ~ m {
        if (match($0, /:[[:space:]]*(.*)$/, a)) {print a[1]; exit}
      }
    ' || true)"
  fi

  # 3) fallback: hyprpaper.conf
  if [[ -z "$p" ]] && [[ -f "$HOME/.config/hypr/hyprpaper.conf" ]]; then
    p="$(awk -F'=' -v m="$mon" '
      /^[[:space:]]*wallpaper[[:space:]]*=/ {
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2)
        split($2, arr, ",")
        monf=arr[1]; gsub(/^[[:space:]]+|[[:space:]]+$/, "", monf)
        path=arr[2]; gsub(/^[[:space:]]+|[[:space:]]+$/, "", path)
        if (monf==m || monf=="") {print path; exit}
      }
    ' "$HOME/.config/hypr/hyprpaper.conf" || true)"
  fi

  printf '%s' "$p"
}

extract_accent_color() {
  local img="$1"
  local color=""

  if command -v magick >/dev/null 2>&1; then
    color="$(magick "$img" -resize 96x96 -colors 10 -format "%c" histogram:info:- 2>/dev/null \
      | sort -nr \
      | awk 'match($0, /#[0-9A-Fa-f]{6}/){print substr($0, RSTART, RLENGTH); exit}' || true)"
  elif command -v convert >/dev/null 2>&1; then
    color="$(convert "$img" -resize 96x96 -colors 10 -format "%c" histogram:info:- 2>/dev/null \
      | sort -nr \
      | awk 'match($0, /#[0-9A-Fa-f]{6}/){print substr($0, RSTART, RLENGTH); exit}' || true)"
  fi

  printf '%s' "$color"
}

extract_wallpaper_luma() {
  local img="$1"
  local luma=""
  if command -v magick >/dev/null 2>&1; then
    luma="$(magick "$img" -resize 1x1\! -colorspace RGB -format "%[fx:(0.2126*r+0.7152*g+0.0722*b)]" info: 2>/dev/null || true)"
  elif command -v convert >/dev/null 2>&1; then
    luma="$(convert "$img" -resize 1x1\! -colorspace RGB -format "%[fx:(0.2126*r+0.7152*g+0.0722*b)]" info: 2>/dev/null || true)"
  fi
  if [[ -z "$luma" ]]; then
    luma="0.5"
  fi
  printf '%s' "$luma"
}

contrast_accent_color() {
  local hex="$1"
  local bg_luma="$2"
  node -e '
    function hexToRgb(h){
      h = String(h || "").trim().replace(/^#/, "");
      if (!/^[0-9a-fA-F]{6}$/.test(h)) return null;
      return [
        parseInt(h.slice(0,2),16)/255,
        parseInt(h.slice(2,4),16)/255,
        parseInt(h.slice(4,6),16)/255
      ];
    }
    function rgbToHex(r,g,b){
      const toHex = (v) => {
        const n = Math.max(0, Math.min(255, Math.round(v * 255)));
        return n.toString(16).padStart(2, "0");
      };
      return "#" + toHex(r) + toHex(g) + toHex(b);
    }
    function rgbToHsl(r,g,b){
      const max = Math.max(r,g,b), min = Math.min(r,g,b);
      let h = 0, s = 0;
      const l = (max + min) / 2;
      if (max !== min) {
        const d = max - min;
        s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
        switch (max) {
          case r: h = (g - b) / d + (g < b ? 6 : 0); break;
          case g: h = (b - r) / d + 2; break;
          default: h = (r - g) / d + 4; break;
        }
        h /= 6;
      }
      return [h,s,l];
    }
    function hslToRgb(h,s,l){
      if (s === 0) return [l,l,l];
      const hue2rgb = (p,q,t) => {
        if (t < 0) t += 1;
        if (t > 1) t -= 1;
        if (t < 1/6) return p + (q - p) * 6 * t;
        if (t < 1/2) return q;
        if (t < 2/3) return p + (q - p) * (2/3 - t) * 6;
        return p;
      };
      const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
      const p = 2 * l - q;
      return [
        hue2rgb(p,q,h + 1/3),
        hue2rgb(p,q,h),
        hue2rgb(p,q,h - 1/3)
      ];
    }
    const color = process.argv[1];
    const bg = Math.max(0, Math.min(1, Number(process.argv[2] || "0.5")));
    const rgb = hexToRgb(color);
    if (!rgb) process.exit(1);
    let [h,s,l] = rgbToHsl(rgb[0], rgb[1], rgb[2]);
    let targetL;
    if (bg < 0.45) targetL = 0.78;
    else if (bg > 0.62) targetL = 0.24;
    else targetL = bg < 0.53 ? 0.68 : 0.32;
    s = Math.max(s, 0.50);
    l = targetL;
    const [r,g,b] = hslToRgb(h,s,l);
    process.stdout.write(rgbToHex(r,g,b).toUpperCase());
  ' "$hex" "$bg_luma" 2>/dev/null || printf '%s' "$hex"
}

echo "[colorwatch] monitor=$MONITOR out=$OUT_FILE interval=${INTERVAL}s mode=${MODE}"

while true; do
  wall="$(extract_wallpaper_path "$MONITOR")"
  if [[ -n "$wall" && -f "$wall" ]]; then
    if [[ "$wall" != "$last_wall" ]]; then
      echo "[colorwatch] wallpaper: $wall"
      last_wall="$wall"
    fi

    c_raw="$(extract_accent_color "$wall")"
    bg_luma="$(extract_wallpaper_luma "$wall")"
    c="$(contrast_accent_color "$c_raw" "$bg_luma")"
    if [[ -n "$c" && "$c" != "$last_color" ]]; then
      printf '%s\n' "$c" > "$OUT_FILE"
      last_color="$c"
      echo "[colorwatch] accent: $c (raw=${c_raw:-na}, bg_luma=${bg_luma})"
    fi
  else
    now_ts="$(date +%s)"
    if (( now_ts - last_warn_ts >= 30 )); then
      echo "[colorwatch] no wallpaper path detectado para monitor=$MONITOR"
      last_warn_ts="$now_ts"
    fi
  fi
  if [[ "$MODE" == "--once" || "$MODE" == "once" ]]; then
    exit 0
  fi
  sleep "$INTERVAL"
done
