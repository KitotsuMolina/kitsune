# Command Reference - Kitsune

Guia actualizada de comandos del entrypoint `kitsune`.

Ruta del proyecto:
`/ruta/a/Kitsune`

## Conceptos clave

- `mode`: familia visual base (`bars` o `ring`).
- `style`: variante de dibujo dentro del `mode`.
- `profile`: tuning de respuesta visual/audio.
- `instance`: stack aislado por monitor.
- `output_target`: destino de render (`mpvpaper` o `layer-shell`), uno activo por instancia.

## CLI Unificado

```bash
kitsune help
kitsune help restart
kitsune status
kitsune doctor
```

## Help y discoverability

- `kitsune help`: lista completa de comandos.
- `kitsune help <comando>`: ayuda puntual (`restart`, `logs`, `doctor`, `config`, `rotate`).
- `kitsune --help`: equivalente a `kitsune help`.

Regla de naming:
- comandos y flags: `kebab-case`
- keys de config: `snake_case`
- sources de logs: `renderer|cava|layer|mpvpaper|colorwatch|monitorwatch`

## Flujo base

### Instalar/preparar entorno
```bash
kitsune install
kitsune install --install-packages
```

Alternativa directa por script:
```bash
./scripts/install.sh
./scripts/install.sh --install-packages
```

Nota:
- `./scripts/install.sh`: bootstrap directo del repo.
- `kitsune install`: entrypoint CLI para el mismo flujo de instalación.

### Iniciar stack completo
```bash
kitsune start
kitsune start eDP-1 --profile ring_video_uno
kitsune start DP-1 --profiles ring_video_uno,ring_video_dos --target layer-shell
```
Hace:
- `cargo build --release`
- crea FIFOs
- inicia frontend de salida (`mpvpaper` o `layer-shell`), `cava`, renderer y watchers opcionales

`start <monitor> ...`:
- crea una instancia aislada por monitor (config/FIFOs/PIDs/logs propios)
- permite elegir perfil(s) en el mismo comando
- `--profile <name>`: perfil fijo (`rotate_profiles=0`)
- `--profiles <p1,p2,...>`: lista para rotacion (`rotate_profiles=1`)
- si no pasas `--mode`, se infiere por prefijo de perfil (`bars*` o `ring*`) cuando aplica

### Detener stack completo
```bash
kitsune stop
kitsune stop eDP-1
```
Hace:
- detiene renderer/cava/frontend de salida/watchers
- borra FIFOs runtime

### Reiniciar stack
```bash
kitsune restart
kitsune restart --rebuild
```
Semantica actual:
- reinicia `output target/cava/renderer/watchers`
- recrea FIFOs runtime
- recompila (porque internamente hace `stop + start`)
- `--rebuild` hoy es compatible y equivale al mismo flujo
- [!] `restart` no es ligero: recompila al usar el mismo flujo que `start`.

## Observabilidad

### Estado operativo
```bash
kitsune status
kitsune status --all-instances
kitsune layer-status
```
Muestra:
- estado de stack (UP o DOWN/PARTIAL)
- backend, spectrum mode, mode/style, runtime mode
- monitor, group file, rotacion, color dinamico
- FIFOs configurados
- PIDs y estado de cada proceso
- rutas de logs
- `fps_real` y `frame_ms` si aparece metrica en log

`layer-status` muestra:
- `output_target` actual
- monitor configurado
- estado del proceso `layer.pid`
- output seleccionado/fallback segun trazas del frontend
- ultimo error de `kitsune-layer` si existe

### Diagnostico guiado
```bash
kitsune doctor
kitsune doctor --fix
kitsune doctor --all-instances
```
`doctor` verifica:
- dependencias requeridas (`cargo`, `rustc`, `cava`, `mpvpaper`, `mpv`)
- dependencias opcionales (`hyprctl`, `kitowall`, `magick`, `convert`)
- existencia de `base.conf` y `cava.conf`
- FIFOs y procesos
- ultimas lineas de logs clave

Contrato `kitowall status`:
- si `dynamic_color=1`: validación requerida (si falla, `doctor` falla)
- si `dynamic_color=0`: validación informativa/no bloqueante

`--fix`:
- limpia PIDs stale
- recrea FIFOs si el stack no esta corriendo
- sincroniza `raw_target` de `cava.conf` con `fifo_cava`

### Logs unificados
```bash
kitsune logs renderer
kitsune logs layer
kitsune logs all --lines 200
kitsune logs cava -f
kitsune logs --all-instances --lines 50
```
Sources:
- `renderer`
- `cava`
- `mpvpaper`
- `layer`
- `colorwatch`
- `monitorwatch`
- `all`

## Config sin editar archivos manualmente

### Leer un valor
```bash
kitsune config get backend
```

### Escribir un valor
```bash
kitsune config set backend gpu
kitsune config set mode ring --restart
kitsune config set dynamic_color 0 --apply
```
`--apply` y `--restart` reinician stack para aplicar cambios.

### Listar configuracion
```bash
kitsune config list
kitsune config list --effective
```
`--effective` agrega datos runtime reales:
- `monitor_selected`, `monitor_real`, `monitor_reason`
- `output_target`
- FIFOs, PIDs y rutas de logs activos
- `test_profile_file`

## Visual

### Recomendado: `visual`
```bash
kitsune visual bars waves_fill
kitsune visual ring dots
```
Cambia `mode` + `style` en un solo comando.

### Comando `style`
```bash
kitsune style bars waves
kitsune style ring waves_fill
```
`style` solo modifica el estilo del modo indicado, no cambia el `mode` global.

### Otros controles visuales
```bash
kitsune mode bars
kitsune mode ring
kitsune wave-roundness 0.85
kitsune ring-fill-softness 0.55
kitsune waves-fill-preset clean
kitsune postfx 1 1 0.18 1.35 0.24 mixed
kitsune particles 1 700 320 0.10 0.28 65 190 1 2 0.70 42 1.40 0.55
kitsune particles-look back '#FFFFFF'
kitsune particles-preset low
kitsune debug overlay 1 --apply
```

`particles` acepta:
- obligatorios: `<enable> <max> <rate> <life_min> <life_max> <speed_min> <speed_max> <size_min> <size_max> <alpha> <drift>`
- opcionales: `[size_scale] [fade_jitter]`

Significado de extras:
- `size_scale`: escala global de tamaño de partículas (`0.2..6.0`)
- `fade_jitter`: aleatoriedad de desaparición/parpadeo (`0..1`)

Nota de `debug overlay`:
- guarda `debug_overlay=0|1` en config
- requiere soporte del renderer para dibujarse en pantalla

## Render y runtime

```bash
kitsune backend cpu
kitsune backend gpu
kitsune output-target mpvpaper
kitsune output-target layer-shell

kitsune spectrum-mode single
kitsune spectrum-mode group
kitsune group-file ./config/groups/default.group
kitsune group validate ./config/groups/default.group
kitsune group list-layers ./config/groups/default.group
kitsune group add-layer "1,bars,bars,bars_balanced,#ffffff,0.35" ./config/groups/default.group
kitsune group update-layer 1 "1,ring,waves_fill,ring_video_uno,#ff2f8f,0.80" ./config/groups/default.group
kitsune group remove-layer 2 ./config/groups/default.group

kitsune runtime standard
kitsune runtime test
```

Nota:
- Solo puede haber un `output_target` activo por instancia.

## Perfiles

### Rotacion
```bash
kitsune rotate 1
kitsune rotate 0
kitsune rotation 10
```

### Control manual de perfil
```bash
kitsune rotate next --apply
kitsune rotate prev --apply
kitsune rotate shuffle
kitsune rotate seed 42
```
`next/prev/shuffle`:
- eligen perfil desde `bars_profiles` o `ring_profiles` segun `mode`
- actualizan `static_profile`
- fuerzan `rotate_profiles=0` para preview manual

### Exploracion de perfiles
```bash
kitsune profiles list
kitsune profiles list bars
kitsune profiles list ring
kitsune profiles show ring_video_uno
kitsune profiles set-list ring ring_video_uno,ring_video_dos
kitsune profiles set-static ring_video_uno
kitsune profiles rotate on
kitsune profiles clone bars_balanced bars_balanced_custom
kitsune profiles set bars_balanced_custom gain 2.2
```

### Edicion/tuning
```bash
kitsune test-load ring_video_uno
kitsune profile-edit gain 2.3
kitsune tune balanced ring
```

## Monitores

```bash
kitsune monitors list
kitsune monitor set DP-1
kitsune monitor-fallback 1 2 1
```

## Color dinamico

```bash
kitsune dynamic-color 1
kitsune color-poll 10
kitsune colorwatch DP-1 /tmp/kitsune-accent.hex 2 --once
```

## Sistema

### Instancias
```bash
kitsune instances list
kitsune instances status eDP-1
kitsune instance-status DP-1
```
- `instances list`: lista instancias runtime detectadas en `./.run/instances/*` y su estado.
- `instances status <monitor>`: detalle de una instancia (config, pids, logs).
- `instance-status <monitor>`: alias corto de `instances status <monitor>`.

### Autostart
```bash
kitsune autostart enable
kitsune autostart enable --monitor DP-1
kitsune autostart status
kitsune autostart status --monitor DP-1
kitsune autostart list
kitsune autostart disable --monitor DP-1
kitsune autostart disable
```
Soporte:
- global: `kitsune.service`
- por monitor: `kitsune@.service` + instancia `kitsune@<monitor>.service`

### Limpieza runtime
```bash
kitsune clean
kitsune clean --force
```
- limpia PIDs/FIFOs/logs runtime
- `--force` detiene stack antes de limpiar

### Reset de config
```bash
kitsune reset
kitsune reset --restart
```
- restaura `config/base.conf` y `config/cava.conf` desde defaults
- crea backup en `config/backups/<timestamp>/`

### Benchmark
```bash
kitsune benchmark
kitsune benchmark 10
```
Reporta:
- backend y fps target
- promedio CPU/RSS del renderer durante la ventana
- `fps_real` si se detecta en logs
