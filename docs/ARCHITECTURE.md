# Architecture - Kitsune

## Objetivo

Renderizar un visualizador de audio para escritorio Wayland/Hyprland que pueda correr como fondo, con control fino de estilos y perfiles.

Pipeline base:
`PulseAudio/PipeWire -> CAVA -> Renderer Rust -> RGBA FIFO -> mpvpaper | layer-shell`

## Bloques principales

### 1. Captura de audio (`cava`)

- `cava` entrega bins en crudo (16-bit LE) por FIFO.
- El renderer lee esos bins continuamente y los normaliza a `0..1`.
- Esta entrada es la fuente única de energía para todos los modos/capas.

### 2. Motor de render (`src/main.rs`)

Responsabilidades:

- Cargar config principal (`config/base.conf`)
- Cargar perfiles (`config/profiles/*.profile`)
- Procesar señal (suavizado temporal/espacial, dinámica, gains por banda)
- Renderizar frame RGBA (CPU o GPU)
- Escribir frame al FIFO de video

## Backends de render

### CPU

- Rasterización por software (rectas, discos, polígonos, etc.).
- Máxima fidelidad con la lógica original.
- Fallback automático si GPU falla.

### GPU (`wgpu`)

- Render offscreen a textura RGBA y readback a buffer.
- Acelera render/composición en escenas complejas.
- Si falla init/render, el engine vuelve a CPU sin detener el servicio.

## Modos de espectro

### `single`

- Un solo espectro activo.
- Usa `mode`, `style`, `runtime_mode`, rotación y/o test profile global.

### `group`

- Varias capas de espectro superpuestas en un solo frame final.
- Cada `layer` puede definir:
  - `mode` y `style`
  - color y alpha
  - runtime por capa (`standard|test|global`)
  - rotación por capa
  - lista de perfiles por capa
  - test file por capa

El grupo vive en `config/groups/*.group`.

## Hot reload

### Single mode

- `test.profile` se recarga sin reinicio.
- Rotación de perfiles por temporizador.

### Group mode

- El archivo `.group` se recarga en caliente (`group_poll_ms`).
- Las capas `runtime=test` recargan su test profile en caliente.
- Las capas `runtime=standard` pueden rotar su lista de perfiles.

## Composición de capas (group)

- Cada capa se renderiza de forma independiente.
- Se mezcla por alpha (`source over`) sobre un framebuffer compuesto.
- El resultado compuesto es el frame final enviado a mpvpaper.

## Color dinámico

- Opcional (`dynamic_color=1`).
- Un watcher externo calcula color acento del wallpaper.
- El renderer aplica transición suave (`color_smooth`) hacia el color objetivo.

## Partículas reactivas

- Emisión para `bars` y `ring` basada en energía de espectro.
- Spawn aleatorio dentro del origen visual:
  - `bars`: área activa de cada barra.
  - `ring`: banda radial + jitter tangencial.
- Tamaño por partícula:
  - rango base (`particles_size_min/max`)
  - modulación por energía
  - escala global (`particles_size_scale`)
- Fade no uniforme:
  - curva de desvanecimiento aleatoria por partícula
  - jitter/flicker configurable (`particles_fade_jitter`)

## Scripts y operación

- `scripts/start.sh`: build + FIFOs + mpvpaper + cava + renderer (+ watcher opcional)
- `scripts/stop.sh`: apagado robusto (SIGTERM + espera + SIGKILL opcional) y borrado de FIFOs runtime
- Scripts de configuración: mutan `config/base.conf` para cambiar modo/backend/rotación/color, etc.

## Archivos de configuración

- `config/base.conf`: configuración global del motor.
- `config/profiles/*.profile`: parámetros DSP y de layout.
- `config/groups/*.group`: definición de capas para `spectrum_mode=group`.

## Razones de diseño

- Mantener CPU como baseline estable y reproducible.
- Permitir GPU opcional sin romper el flujo existente.
- Separar visual (`mode/style`) de comportamiento (`profile/runtime`).
- Habilitar iteración rápida (hot reload) sin reiniciar todo el stack.
