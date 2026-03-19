# GTK4 Layer Migration

## Goal

Replace the current pipeline:

`PulseAudio/PipeWire -> CAVA -> renderer -> RGBA FIFO -> kitsune-layer | mpvpaper`

with:

`audio frame -> overlay draw directly`

using `gtk4-layer-shell`.

## Why

`group` mode is currently limited by architecture, not just by configuration.

The existing flow incurs:

- per-layer render into a full RGBA framebuffer
- GPU to CPU readback for GPU mode
- full-frame alpha compositing
- continuous FIFO video writes
- a second frontend process that only consumes pixels

This is the wrong shape for responsive overlay spectrums.

## Target Architecture

### New frontend

- single overlay process
- `gtk4-layer-shell` for compositor integration
- direct drawing to a `DrawingArea`
- no RGBA video FIFO
- no `kitsune-layer` pixel-consumer role

### Audio path

- `cava` or `PipeWire` sampled directly into memory
- latest spectrum frame stored in-process
- UI ticks repaint against the latest normalized bars

### Rendering path

- bars, waves, ring, polygon/triangle drawn directly in overlay
- group composition performed in draw/update logic, not in fullscreen video buffers

## Migration Order

1. Introduce `kitsune-overlay` based on `gtk4-layer-shell`
2. Move direct audio frame ingestion into the overlay process
3. Port line/bar visuals first
4. Port radial/ring and polygon layouts
5. Port group composition without RGBA/FIFO roundtrips
6. Remove `kitsune-layer`
7. Remove `mpvpaper` references and package dependencies
8. Simplify scripts/docs/CLI around a single overlay target

## Current Status

- `kitsune-overlay` introduced as the new frontend base
- full switchover not complete yet
- old pipeline still exists until scripts and CLI are rewired
