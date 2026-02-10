_kitsune_profiles() {
  if command -v kitsune >/dev/null 2>&1; then
    kitsune profiles list all 2>/dev/null
  elif [[ -d ./config/profiles ]]; then
    ls ./config/profiles/*.profile 2>/dev/null | xargs -n1 basename | sed 's/\.profile$//'
  fi
}

_kitsune_monitors() {
  if command -v hyprctl >/dev/null 2>&1; then
    hyprctl monitors 2>/dev/null | awk '/^Monitor /{print $2}'
  fi
}

_kitsune_group_files() {
  ls ./config/groups/*.group 2>/dev/null | xargs -n1 basename
}

_kitsune_completion() {
  local cur prev words cword
  _init_completion -n : || return

  local commands="install start stop restart status doctor run logs layer-status config visual style mode backend output-target spectrum-mode group-file group runtime rotate rotation profiles test-load profile-edit tune dynamic-color color-poll colorwatch postfx particles particles-look particles-preset monitors monitor monitor-fallback autostart instances instance-status clean reset benchmark help"

  if [[ $cword -eq 1 ]]; then
    COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
    return
  fi

  case "${words[1]}" in
    help)
      COMPREPLY=( $(compgen -W "restart logs layer-status doctor config rotate instances" -- "$cur") )
      ;;
    start)
      if [[ "$prev" == "--profile" || "$prev" == "--profiles" ]]; then
        COMPREPLY=( $(compgen -W "$(_kitsune_profiles)" -- "$cur") )
      elif [[ "$prev" == "--monitor" ]]; then
        COMPREPLY=( $(compgen -W "$(_kitsune_monitors)" -- "$cur") )
      elif [[ "$prev" == "--target" ]]; then
        COMPREPLY=( $(compgen -W "mpvpaper layer-shell" -- "$cur") )
      elif [[ "$prev" == "--mode" ]]; then
        COMPREPLY=( $(compgen -W "bars ring" -- "$cur") )
      else
        COMPREPLY=( $(compgen -W "--monitor --profile --profiles --target --mode $(_kitsune_monitors)" -- "$cur") )
      fi
      ;;
    stop)
      if [[ "$prev" == "--monitor" ]]; then
        COMPREPLY=( $(compgen -W "$(_kitsune_monitors)" -- "$cur") )
      else
        COMPREPLY=( $(compgen -W "--monitor $(_kitsune_monitors)" -- "$cur") )
      fi
      ;;
    logs)
      COMPREPLY=( $(compgen -W "renderer cava mpvpaper layer colorwatch monitorwatch all --lines -f --follow --all-instances" -- "$cur") )
      ;;
    status)
      COMPREPLY=( $(compgen -W "--all-instances" -- "$cur") )
      ;;
    doctor)
      COMPREPLY=( $(compgen -W "--fix --all-instances" -- "$cur") )
      ;;
    config)
      COMPREPLY=( $(compgen -W "get set list --effective --apply --restart" -- "$cur") )
      ;;
    visual)
      if [[ ${cword} -eq 2 ]]; then
        COMPREPLY=( $(compgen -W "bars ring" -- "$cur") )
      else
        COMPREPLY=( $(compgen -W "bars bars_fill waves waves_fill dots" -- "$cur") )
      fi
      ;;
    style)
      if [[ ${cword} -eq 2 ]]; then
        COMPREPLY=( $(compgen -W "bars ring" -- "$cur") )
      else
        COMPREPLY=( $(compgen -W "bars bars_fill waves waves_fill dots" -- "$cur") )
      fi
      ;;
    mode)
      COMPREPLY=( $(compgen -W "bars ring" -- "$cur") )
      ;;
    backend)
      COMPREPLY=( $(compgen -W "cpu gpu" -- "$cur") )
      ;;
    output-target)
      COMPREPLY=( $(compgen -W "mpvpaper layer-shell" -- "$cur") )
      ;;
    spectrum-mode)
      COMPREPLY=( $(compgen -W "single group" -- "$cur") )
      ;;
    group-file)
      COMPREPLY=( $(compgen -W "$(_kitsune_group_files)" -- "$cur") )
      ;;
    group)
      if [[ ${cword} -eq 2 ]]; then
        COMPREPLY=( $(compgen -W "validate list-layers add-layer update-layer remove-layer" -- "$cur") )
      else
        COMPREPLY=( $(compgen -W "$(_kitsune_group_files)" -- "$cur") )
      fi
      ;;
    runtime)
      COMPREPLY=( $(compgen -W "standard test" -- "$cur") )
      ;;
    rotate)
      COMPREPLY=( $(compgen -W "0 1 next prev shuffle seed --apply" -- "$cur") )
      ;;
    profiles)
      if [[ ${cword} -eq 2 ]]; then
        COMPREPLY=( $(compgen -W "list show set-list set-static rotate clone set" -- "$cur") )
      else
        COMPREPLY=( $(compgen -W "$(_kitsune_profiles) bars ring all on off" -- "$cur") )
      fi
      ;;
    test-load)
      COMPREPLY=( $(compgen -W "$(_kitsune_profiles)" -- "$cur") )
      ;;
    monitors)
      COMPREPLY=( $(compgen -W "list" -- "$cur") )
      ;;
    monitor)
      if [[ ${cword} -eq 2 ]]; then
        COMPREPLY=( $(compgen -W "set" -- "$cur") )
      else
        COMPREPLY=( $(compgen -W "$(_kitsune_monitors)" -- "$cur") )
      fi
      ;;
    monitor-fallback)
      COMPREPLY=( $(compgen -W "0 1" -- "$cur") )
      ;;
    autostart)
      COMPREPLY=( $(compgen -W "enable disable status list --monitor $(_kitsune_monitors)" -- "$cur") )
      ;;
    instances)
      if [[ ${cword} -eq 2 ]]; then
        COMPREPLY=( $(compgen -W "list status" -- "$cur") )
      else
        COMPREPLY=( $(compgen -W "$(_kitsune_monitors)" -- "$cur") )
      fi
      ;;
    instance-status)
      COMPREPLY=( $(compgen -W "$(_kitsune_monitors)" -- "$cur") )
      ;;
    clean)
      COMPREPLY=( $(compgen -W "--force" -- "$cur") )
      ;;
    reset)
      COMPREPLY=( $(compgen -W "--restart" -- "$cur") )
      ;;
  esac
}

complete -F _kitsune_completion kitsune
