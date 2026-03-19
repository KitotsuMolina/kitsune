# Host Install Repair

Este documento deja registrado un fallo de instalacion de host que ya fue corregido en `Kitowall/bootstrap-host.sh`, pero que puede aparecer en instalaciones viejas.

## Sintoma

Al ejecutar `kitsune` instalado en host:

```bash
kitsune stop
kitsune doctor --fix
kitsune start eDP-1 --target layer-shell --mode bars
```

pueden aparecer errores como:

```text
./scripts/stop.sh: No existe el fichero o el directorio
cp: no se puede efectuar `stat' sobre './config/base.conf'
fork: retry: Recurso no disponible temporalmente
```

o `doctor` puede mostrar procesos stale y el espectro no aparece aunque el flujo diga `Running`.

## Causa

Hubo una instalacion de host defectuosa donde:

1. `~/.local/bin/kitsune` terminaba apuntando o sobrescribiendo rutas internas incorrectas.
2. `~/.local/share/kitsune/bin/kitsune` dejaba de ser el binario ELF real y terminaba siendo un shell wrapper.
3. En algunos casos `~/.local/share/kitsune/scripts/kitsune.sh` tambien quedaba sobrescrito por el wrapper, generando recursion.

El resultado era que `kitsune` de host ya no encontraba bien:

- `scripts/`
- `config/`
- `bin/kitsune`
- runtime state

## Verificacion

El estado correcto es este:

```bash
file ~/.local/share/kitsune/bin/kitsune
file ~/.local/share/kitsune/bin/kitsune-layer
head -n 12 ~/.local/bin/kitsune
head -n 5 ~/.local/share/kitsune/scripts/kitsune.sh
```

Esperado:

- `~/.local/share/kitsune/bin/kitsune` => `ELF`
- `~/.local/share/kitsune/bin/kitsune-layer` => `ELF`
- `~/.local/bin/kitsune` => wrapper corto
- `~/.local/share/kitsune/scripts/kitsune.sh` => script real de Kitsune, no wrapper

## Repair

Si el host esta en ese estado roto, ejecutar desde el repo de `Kitowall`:

```bash
cd /home/kitotsu/Programacion/Personal/Wallpaper/Kitowall
KITOWALL_BOOTSTRAP_MODE=kitsune-repair ./scripts/bootstrap-host.sh
```

Si quieres forzar descarga de bins otra vez:

```bash
cd /home/kitotsu/Programacion/Personal/Wallpaper/Kitowall
KITOWALL_BOOTSTRAP_MODE=kitsune-only ./scripts/bootstrap-host.sh
```

## Layout correcto en host

La instalacion esperada queda asi:

- `~/.local/bin/kitsune` -> wrapper de entrada
- `~/.local/share/kitsune/bin/kitsune` -> binario real
- `~/.local/share/kitsune/bin/kitsune-layer` -> binario real
- `~/.local/share/kitsune/scripts/` -> scripts de Kitsune
- `~/.config/kitsune/base.conf` -> config de usuario
- `~/.config/kitsune/cava.conf` -> config de usuario
- `~/.local/state/kitsune/run` -> estado runtime

## Nota

Este fallo no deberia seguir ocurriendo una vez que el bootstrap corregido de `Kitowall` este desplegado y el host se reinstale o repare una vez.
