# Kitsune Status
Fecha de snapshot: 2026-02-09

## Estado general
`Kitsune` está en estado **funcional avanzado** para Hyprland/Wayland, con:
- renderer Rust (`cpu|gpu`)
- salida `mpvpaper` y salida nativa `layer-shell`
- CLI unificado amplio
- multi-instancia por monitor
- validación smoke automatizada

## Pipeline actual
`PulseAudio/PipeWire -> CAVA -> renderer (CPU/GPU) -> RGBA FIFO -> mpvpaper | layer-shell`

## Funcionalidad implementada
- Arranque/parada/restart del stack.
- Configuración por comandos (`config get/set/list`).
- Observabilidad: `status`, `doctor`, `logs`, `layer-status`.
  - soporte agregado: `status --all-instances`, `doctor --all-instances`, `logs --all-instances`
- Visual: `mode/style/visual`, postfx, partículas.
- Perfiles:
  - listar/mostrar
  - set-list, set-static, rotate on/off
  - clone y set de claves
- Group:
  - validate
  - list-layers
  - add/update/remove layer
- Monitores:
  - `monitors list`, `monitor set`
  - fallback de monitor (en target `mpvpaper`)
- Multi-instancia por monitor:
  - `start <monitor> --profile|--profiles ...`
  - `stop <monitor>`
  - `instances list`, `instances status <monitor>`
- Completions:
  - bash, zsh, fish
- Instalador:
  - `install.sh --install-packages`
  - `install.sh --install-completions`

## Estado de calidad
- `cargo check --bins`: OK
- Smoke tests CLI (`tests/cli-smoke.sh`): OK
- Smoke tests full (`tests/cli-smoke.sh --full`): OK (con skip esperado para permisos de autostart en sandbox)
- Release gate script: `scripts/release-check.sh`

## Contratos v1 congelados

### Contrato con `kitowall status`
`Kitsune` usa como contrato:
- comando disponible: `kitowall` en `PATH`
- ejecución válida: `kitowall status` con exit 0
- payload JSON válido
- campo `last_set` objeto
- lookup de wallpaper por monitor: `last_set[monitor]` (exacto o case-insensitive)

En `doctor`:
- **hard check** cuando `dynamic_color=1`
- **informativo/no bloqueante** cuando `dynamic_color=0`

### Convenciones de naming
- comandos y flags CLI: `kebab-case`
- keys de config: `snake_case`
- sources de logs (fijos): `renderer|cava|layer|mpvpaper|colorwatch|monitorwatch`

## Decisiones recientes importantes
- `output_target` configurable (`mpvpaper|layer-shell`).
- `layer-shell` en `Layer::Bottom`.
- Pin de output por nombre de monitor implementado en `kitsune-layer`.
- `kitsune-layer` recrea buffers en caliente ante `configure` con cambio de tamaño.
- Instancias aisladas usan config/FIFOs/PIDs/logs separados.
- `cava.conf` por instancia para evitar pisar estado global.
- Partículas con spawn/origen aleatorio, `particles_size_scale` y `particles_fade_jitter`.
- `stop.sh` con parada más robusta (espera tras SIGTERM y SIGKILL opcional).

## Limitaciones conocidas
- `layer-shell` depende del compositor/sesión Wayland activa; en entornos sin compositor puede fallar con `NoCompositor`.
- El flujo principal (`status`, `doctor`) sigue orientado a la instancia global; para instancias usar `instances status <monitor>`.
- Gestión de capas `group` via CLI usa entradas CSV manuales (válido, pero sin asistente interactivo).

## Próximos pasos sugeridos
1. Test e2e visual Wayland (con compositor real en CI o runner dedicado).
2. Validación semántica extendida de `group` (actual ya cubre alpha/color/mode/style/profile/rotate+profiles_pipe/id duplicado).
3. Vista agregada de métricas de performance por instancia (`benchmark --all-instances`).
4. Export/import de presets de perfiles y grupos.
