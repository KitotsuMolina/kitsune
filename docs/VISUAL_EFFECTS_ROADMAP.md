# Kitsune Visual Effects Roadmap

Este documento lista los efectos visuales que encajan bien con `Kitsune` y su estado recomendado.

## Prioridad Alta

- `neon_glow`
  - Halo suave y aditivo alrededor del espectro.
  - Bajo costo y alto impacto visual.
- `afterglow`
  - Rastro corto de frames anteriores.
  - Hace que el movimiento se sienta mas fluido.
- `free_particles`
  - Particulas decorativas no ligadas al audio.
  - En `bars`: nacen desde la base y suben aleatoriamente.
  - En `ring`: nacen en el centro y salen hacia afuera aleatoriamente.
- `dual_gradient`
  - Gradiente primario/secundario mas evidente.
  - Especialmente util para `waves` y `ring`.

## Prioridad Media

- `particle_burst`
  - Emision por transientes o picos.
- `ghost_echo`
  - Eco visual mas marcado que `afterglow`.
- `scanline_shimmer`
  - Barrido de brillo sutil sobre la forma.
- `orbit_particles`
  - Particulas orbitales para `ring`.
- `segmented_neon`
  - Barras segmentadas con glow fuerte.

## Prioridad Baja

- `heat_haze`
  - Distorsion ligera alrededor del espectro.
- `chromatic_aberration`
  - Separacion RGB en bordes.
- `liquid_displacement`
  - Deformacion tipo fluido.
- `noise_field`
  - Campo de ruido reactivo.
- `metaballs`
  - Blobs reactivos de alto costo.

## Recomendacion De Implementacion

Orden sugerido:

1. `neon_glow`
2. `afterglow`
3. `free_particles`
4. `dual_gradient`
5. `particle_burst`

## Configuracion Deseable

Parametros recomendados para exponer en `Studio`:

- `neon_enabled`
- `neon_strength`
- `neon_layers`
- `afterglow_enabled`
- `afterglow_decay`
- `afterglow_alpha`
- `particles_enabled`
- `particles_spawn_rate`
- `particles_max`
- `particles_mode`
  - `bars_base`
  - `ring_center`
- `particles_alpha`
- `particles_speed_min`
- `particles_speed_max`
- `particles_size_min`
- `particles_size_max`
- `particles_drift`

## Estado Actual

Ruta nueva recomendada en el overlay GTK:

- `neon_glow`: implementado
  - varias pasadas de dibujo con alpha decreciente
  - usa `neon_enabled`, `neon_strength`, `neon_layers`
- `afterglow`: implementado
  - copia amortiguada del espectro previo
  - usa `afterglow_enabled`, `afterglow_decay`, `afterglow_alpha`
- `free_particles`: implementado
  - no depende de la musica
  - en `bars`: nacen desde la base y suben aleatoriamente
  - en `ring`: nacen desde el centro y salen hacia afuera aleatoriamente
  - usa `particles_enabled`, `particles_spawn_rate`, `particles_mode`

## Efectos Actualmente Disponibles

- `neon_glow`
- `afterglow`
- `free_particles`
- `dual_gradient`
  - ya estaba presente de forma basica a traves de `color` y `color2`

## Proximo Bloque Recomendado

Los siguientes efectos siguen siendo los mas rentables para continuar:

1. `particle_burst`
2. `ghost_echo`
3. `scanline_shimmer`
4. `orbit_particles`
5. `segmented_neon`

La prioridad debe mantenerse en efectos que:

- no rompan el rendimiento
- funcionen bien tanto en `bars` como en `ring`
- puedan combinarse con `group`
