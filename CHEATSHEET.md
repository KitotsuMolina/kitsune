# Cheat Sheet - Kitsune

Ruta:
`/ruta/a/Kitsune`

## Arranque y parada

```bash
kitsune start
kitsune stop
```

## Modos base

```bash
kitsune backend cpu
kitsune backend gpu
```

```bash
kitsune spectrum-mode single
kitsune spectrum-mode group
kitsune group-file ./config/groups/default.group
```

```bash
kitsune mode bars
kitsune mode ring
```

```bash
kitsune visual bars waves
kitsune visual bars bars_fill
kitsune visual bars waves_fill
kitsune visual bars dots
kitsune visual ring waves
kitsune visual ring waves_fill
kitsune visual ring dots
```

```bash
kitsune style bars bars
kitsune style bars bars_fill
kitsune style bars waves
kitsune style bars waves_fill
kitsune style bars dots
kitsune style ring bars
kitsune style ring waves
kitsune style ring waves_fill
kitsune style ring dots
```

Para waves mas redondeadas (editando `config/base.conf`):
```ini
bars_wave_roundness=0.85
ring_wave_roundness=0.85
```

O con script:
```bash
kitsune wave-roundness 0.85
```

Suavidad del relleno en ring `waves_fill`:
```bash
kitsune ring-fill-softness 0.55
```

Presets rápidos de `waves_fill` (ring):
```bash
kitsune waves-fill-preset clean
kitsune waves-fill-preset impact
```

```bash
kitsune runtime standard
kitsune runtime test
```

- `standard`: usa perfil fijo o rotación.
- `test`: usa `test.profile` con recarga en caliente.

## Rotación de perfiles

```bash
kitsune rotate 1   # activar
kitsune rotate 0   # desactivar
kitsune rotation 10         # cada 10s
```

## Color dinámico por wallpaper

```bash
kitsune dynamic-color 1
kitsune color-poll 10
```

Manual watcher:

```bash
./scripts/wallpaper-accent-watcher.sh DP-1 /tmp/kitsune-accent.hex 2
```

Desactivar color dinámico:

```bash
kitsune dynamic-color 0
```

## Modo test (edición en caliente)

```bash
kitsune runtime test
kitsune test-load ring_video_uno
kitsune profile-edit gain 2.3
kitsune profile-edit low_band_gain 1.8
kitsune profile-edit high_band_gain 0.9
```

## Presets rápidos

```bash
kitsune tune soft ring
kitsune tune bass-heavy ring
kitsune tune vocal bars
```

Presets disponibles:
- `soft`
- `punchy`
- `bass-heavy`
- `vocal`
- `balanced`
- `cinematic`
- `energetic`

## Logs útiles

```bash
tail -f /tmp/kitsune-renderer.log
tail -f /tmp/kitsune-cava.log
tail -f /tmp/kitsune-mpvpaper.log
tail -f /tmp/kitsune-colorwatch.log
```

## Partículas (nuevo flujo)

Config rápida:

```bash
kitsune particles 1 700 320 0.10 0.28 65 190 1 2 0.70 42 1.40 0.55
kitsune particles-look back '#FFFFFF'
kitsune restart
```

- `size_scale` (arg 12, opcional): escala de tamaño global (`0.2..6.0`)
- `fade_jitter` (arg 13, opcional): aleatoriedad de fade/parpadeo (`0..1`)

Presets recomendados (antes -> después):

```bash
# low
# antes:    kitsune particles 1 320 140 0.10 0.20 80 220 1 2 0.58 36
# después:  kitsune particles 1 320 140 0.10 0.20 80 220 1 2 0.58 36 1.15 0.35

# balanced
# antes:    kitsune particles 1 520 260 0.12 0.28 95 280 1 2 0.66 42
# después:  kitsune particles 1 520 260 0.12 0.28 95 280 1 2 0.66 42 1.35 0.50

# high
# antes:    kitsune particles 1 1000 520 0.16 0.42 120 360 1 3 0.76 54
# después:  kitsune particles 1 1000 520 0.16 0.42 120 360 1 3 0.76 54 1.65 0.70
```

## Recetas rápidas

Ring con rotación + color dinámico:

```bash
kitsune mode ring
kitsune runtime standard
kitsune rotate 1
kitsune rotation 10
kitsune dynamic-color 1
kitsune color-poll 10
kitsune restart
```

Cambiar estilo y aplicar reiniciando:

```bash
cd /ruta/a/Kitsune
kitsune style bars waves
kitsune style bars waves_fill
kitsune style ring dots
kitsune restart
```

Tuning en caliente:

```bash
kitsune runtime test
kitsune test-load ring_video_uno
kitsune restart
kitsune profile-edit gain 2.4
kitsune profile-edit mid_band_gain 1.5
```

Perfil estático + color fijo:

```bash
kitsune runtime standard
kitsune rotate 0
kitsune dynamic-color 0
kitsune restart
```
