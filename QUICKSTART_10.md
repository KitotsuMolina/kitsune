# Quickstart Comandos

Ruta:
`/ruta/a/Kitsune`

```bash
# 1) Iniciar todo
kitsune start

# 2) Detener todo
kitsune stop

# 3) Cambiar a barras
kitsune mode bars

# 4) Cambiar a ring
kitsune mode ring

# 5) Cambiar backend (cpu o gpu)
kitsune backend gpu

# 6) Cambiar modo espectro (normal o grupo)
kitsune spectrum-mode single
kitsune spectrum-mode group
kitsune group-file ./config/groups/default.group

# 7) Modo normal (perfil fijo/rotación)
kitsune runtime standard

# 8) Modo prueba (hot reload)
kitsune runtime test

# 9) Activar rotación de perfiles
kitsune rotate 1

# 10) Rotar cada 10 segundos
kitsune rotation 10

# 11) Activar color dinámico por wallpaper
kitsune dynamic-color 1

# 12) Cargar perfil base en test y empezar a tunear
kitsune test-load ring_video_uno

# 13) Activar partículas con tamaño/fade aleatorio ajustable
kitsune particles 1 700 320 0.10 0.28 65 190 1 2 0.70 42 1.40 0.55
```

Edición rápida en caliente (extra):

```bash
kitsune profile-edit gain 2.3
kitsune profile-edit low_band_gain 1.8
kitsune profile-edit high_band_gain 0.9
```

Estilos (extra):

```bash
kitsune visual bars waves
kitsune visual bars bars_fill
kitsune visual bars waves_fill
kitsune visual ring dots
kitsune visual ring waves_fill
kitsune wave-roundness 0.85
kitsune ring-fill-softness 0.55
kitsune waves-fill-preset clean
kitsune waves-fill-preset impact
kitsune style bars waves
kitsune style bars bars_fill
kitsune style bars waves_fill
kitsune style bars dots
kitsune style ring waves
kitsune style ring waves_fill
kitsune style ring dots
```

Aplicar cambios de estilo:

```bash
kitsune restart
```
