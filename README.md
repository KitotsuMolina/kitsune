# Kitsune

Visualizador de audio para Hyprland/Wayland, escrito en Rust.

Pipeline:
`PulseAudio/PipeWire -> CAVA -> renderer (CPU/GPU) -> RGBA FIFO -> mpvpaper | layer-shell`

## Visión

Este proyecto busca un visualizador de escritorio estable y configurable, con:

- backend seleccionable (`cpu` o `gpu`)
- espectro normal (`single`) o superposición de capas (`group`)
- estilos visuales (`bars`, `waves`, `waves_fill`, `dots`)
- perfiles editables y hot-reload
- color dinámico basado en wallpaper

## Conceptos clave

- `mode`: familia visual base (`bars` o `ring`).
- `style`: variante de dibujo dentro del `mode`.
- `profile`: tuning de respuesta visual/audio.
- `instance`: stack aislado por monitor.
- `output_target`: destino de render (`mpvpaper` o `layer-shell`), uno activo por instancia.

## Uso básico

Instalación/configuración inicial (recomendado):

```bash
cd /ruta/a/Kitsune
./scripts/install.sh
```

Si quieres que también intente instalar paquetes del sistema:

```bash
./scripts/install.sh --install-packages
```

Si además quieres instalar completions de shell durante install:

```bash
./scripts/install.sh --install-completions
```

Alternativa desde CLI:

```bash
kitsune install
kitsune install --install-packages
```

Nota:
- `./scripts/install.sh`: bootstrap directo del repo.
- `kitsune install`: entrypoint CLI para el mismo flujo de instalación.

CLI unificado (recomendado):

```bash
kitsune help
kitsune help restart
kitsune start
kitsune status
kitsune doctor
kitsune logs all --lines 120
kitsune visual bars bars_fill
kitsune colorwatch DP-1 /tmp/kitsune-accent.hex 2 --once
kitsune rotate next --apply
kitsune particles-preset low
kitsune stop
```

Iniciar:

```bash
kitsune start
kitsune start eDP-1 --profile ring_video_uno
kitsune start DP-1 --profiles ring_video_uno,ring_video_dos --target layer-shell
```

Detener:

```bash
kitsune stop
kitsune stop eDP-1
```

Seleccionar backend:

```bash
kitsune backend cpu
kitsune backend gpu
```

Seleccionar target de salida:

```bash
kitsune output-target mpvpaper
kitsune output-target layer-shell
kitsune restart
```

Nota:
- Solo puede haber un `output_target` activo por instancia.

Seleccionar modo de espectro:

```bash
kitsune spectrum-mode single
kitsune spectrum-mode group
kitsune group-file ./config/groups/default.group
```

Fallback automático de monitor (cuando desconectas/reconectas pantallas):

```bash
kitsune monitor-fallback 1 2 1
```

Cambiar visual rápida:

```bash
kitsune visual ring waves_fill
kitsune restart
```

Nota sobre `restart`:
- Reinicia stack completo (target de salida `mpvpaper|layer-shell`, `cava`, renderer y watchers).
- Recrea FIFOs runtime.
- [!] `restart` recompila porque usa el mismo flujo que `start`.

Efectos postproceso (glow + blur):

```bash
kitsune postfx 1 1 0.18 1.35 0.24 mixed
kitsune restart
```

Partículas reactivas (bars/ring):

```bash
kitsune particles 1 700 320 0.10 0.28 65 190 1 2 0.70 42 1.40 0.55
kitsune particles-look back '#FFFFFF'
kitsune restart
```

Parámetros extra de `particles`:
- `size_scale` (opcional): multiplicador global de tamaño (`0.2..6.0`).
- `fade_jitter` (opcional): variación aleatoria de desaparición/parpadeo (`0..1`).

Comportamiento actual de partículas:
- Spawn desde puntos aleatorios del origen visual (área de barra o banda radial del ring).
- Tamaño con variación por energía + escala global.
- Desvanecimiento no uniforme (curvas aleatorias por partícula).

## Validacion de comandos (smoke tests)

Suite CLI:

```bash
./tests/cli-smoke.sh
./tests/cli-smoke.sh --full
```

- `safe` (default): valida comandos sin requerir acciones de sistema persistentes.
- `full`: agrega `restart`, `benchmark`, `autostart`, `clean`, `reset`, `start`, `stop`.

## Release check

```bash
./scripts/release-check.sh
```

Ejecuta:
- `cargo check --bins`
- `./tests/cli-smoke.sh --full`

## Autocompletado (bash/zsh/fish)

Instalar completions:

```bash
./scripts/install-completions.sh
# o individual:
./scripts/install-completions.sh bash
./scripts/install-completions.sh zsh
./scripts/install-completions.sh fish
```

Archivos:
- bash: `completions/kitsune.bash`
- zsh: `completions/_kitsune`
- fish: `completions/kitsune.fish`

## Multi-instancia por monitor

- `kitsune start <monitor> ...` crea/usa una instancia aislada por monitor.
- Cada instancia usa config runtime propia en `./.run/instances/<monitor_sanitizado>/config/base.conf`.
- Cada instancia usa FIFOs y logs dedicados (`/tmp/kitsune-<id>-*.log`).

Comandos utiles:

```bash
kitsune start eDP-1 --profile ring_video_uno
kitsune start DP-1 --profiles bars_balanced,bars_punchy --mode bars --target mpvpaper
kitsune instances list
kitsune instances status eDP-1
kitsune instance-status DP-1
kitsune status --all-instances
kitsune doctor --all-instances
kitsune logs --all-instances --lines 50
kitsune stop eDP-1
kitsune stop DP-1
```

## Estructura de documentación

- `docs/COMMANDS.md`: referencia completa de comandos y configuraciones.
- `docs/ARCHITECTURE.md`: arquitectura, decisiones y responsabilidades de cada módulo.
- `CHEATSHEET.md`: comandos frecuentes.
- `QUICKSTART_10.md`: flujo rápido diario.
- `LICENSE.md`: licencia del proyecto (MIT + nota de marcas).
