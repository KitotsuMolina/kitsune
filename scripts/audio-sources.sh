#!/usr/bin/env bash
set -euo pipefail

json=0
if [[ "${1:-}" == "--json" ]]; then
  json=1
fi

json_escape() {
  local s=${1:-}
  s=${s//\\/\\\\}
  s=${s//"/\\"}
  s=${s//$'\n'/\\n}
  s=${s//$'\r'/}
  s=${s//$'\t'/\\t}
  printf '%s' "$s"
}

slugify() {
  local value
  value=$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//')
  if [[ -z "$value" ]]; then
    value="unknown"
  fi
  printf '%s' "$value"
}

if ! command -v pactl >/dev/null 2>&1; then
  if [[ $json -eq 1 ]]; then
    printf '{"ok":false,"error":"pactl not found","apps":[],"streams":[]}\n'
  else
    echo "[x] pactl no disponible"
  fi
  exit 0
fi

raw="$(pactl list sink-inputs 2>/dev/null || true)"
if [[ -z "$raw" ]]; then
  if [[ $json -eq 1 ]]; then
    printf '{"ok":true,"apps":[],"streams":[]}\n'
  else
    echo "[i] No hay streams de salida activos"
  fi
  exit 0
fi

declare -A app_label_by_id=()
declare -A app_binary_by_id=()
declare -A app_count_by_id=()
declare -a app_order=()
declare -a stream_rows=()

index=""
app_name=""
binary=""
media_name=""
node_name=""
object_id=""

flush_record() {
  if [[ -z "$index" ]]; then
    return
  fi
  local label="$app_name"
  if [[ -z "$label" ]]; then
    label="$node_name"
  fi
  if [[ -z "$label" ]]; then
    label="Stream $index"
  fi
  local slug
  slug=$(slugify "$label")
  local app_id="app:$slug"
  if [[ -z "${app_label_by_id[$app_id]+x}" ]]; then
    app_label_by_id[$app_id]="$label"
    app_binary_by_id[$app_id]="$binary"
    app_count_by_id[$app_id]=0
    app_order+=("$app_id")
  fi
  app_count_by_id[$app_id]=$(( ${app_count_by_id[$app_id]} + 1 ))
  stream_rows+=("$index|$app_id|$label|$binary|$media_name|$node_name|$object_id")
  index=""
  app_name=""
  binary=""
  media_name=""
  node_name=""
  object_id=""
}

while IFS= read -r line; do
  if [[ $line =~ ^Sink\ Input\ \#([0-9]+)$ ]]; then
    flush_record
    index="${BASH_REMATCH[1]}"
    continue
  fi
  [[ -n "$index" ]] || continue
  case "$line" in
    *'application.name = "'*)
      app_name="${line#*application.name = \"}"
      app_name="${app_name%\"}"
      ;;
    *'application.process.binary = "'*)
      binary="${line#*application.process.binary = \"}"
      binary="${binary%\"}"
      ;;
    *'media.name = "'*)
      media_name="${line#*media.name = \"}"
      media_name="${media_name%\"}"
      ;;
    *'node.name = "'*)
      node_name="${line#*node.name = \"}"
      node_name="${node_name%\"}"
      ;;
    *'object.id = "'*)
      object_id="${line#*object.id = \"}"
      object_id="${object_id%\"}"
      ;;
  esac
done <<< "$raw"
flush_record

if [[ $json -eq 0 ]]; then
  echo "Apps disponibles:"
  for app_id in "${app_order[@]}"; do
    echo "- ${app_label_by_id[$app_id]} (${app_id}, streams=${app_count_by_id[$app_id]})"
  done
  if [[ ${#stream_rows[@]} -gt 0 ]]; then
    echo
    echo "Streams activos:"
    for row in "${stream_rows[@]}"; do
      IFS='|' read -r row_index row_app_id row_label row_binary row_media row_node row_object <<< "$row"
      echo "- #$row_index app=$row_label match=$row_app_id media=${row_media:-Playback} node=${row_node:-$row_label} object=${row_object:-?}"
    done
  fi
  exit 0
fi

printf '{"ok":true,"apps":['
app_sep=""
for app_id in "${app_order[@]}"; do
  printf '%s{"id":"%s","label":"%s","binary":"%s","streamCount":%s}' \
    "$app_sep" \
    "$(json_escape "$app_id")" \
    "$(json_escape "${app_label_by_id[$app_id]}")" \
    "$(json_escape "${app_binary_by_id[$app_id]}")" \
    "${app_count_by_id[$app_id]}"
  app_sep=","
done
printf '],"streams":['
stream_sep=""
for row in "${stream_rows[@]}"; do
  IFS='|' read -r row_index row_app_id row_label row_binary row_media row_node row_object <<< "$row"
  printf '%s{"id":"stream:%s","index":%s,"appId":"%s","label":"%s","binary":"%s","mediaName":"%s","nodeName":"%s","objectId":"%s"}' \
    "$stream_sep" \
    "$(json_escape "$row_index")" \
    "$row_index" \
    "$(json_escape "$row_app_id")" \
    "$(json_escape "$row_label")" \
    "$(json_escape "$row_binary")" \
    "$(json_escape "$row_media")" \
    "$(json_escape "$row_node")" \
    "$(json_escape "$row_object")"
  stream_sep=","
done
printf ']}\n'
