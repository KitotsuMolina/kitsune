function __kitsune_monitors
    if type -q hyprctl
        hyprctl monitors 2>/dev/null | awk '/^Monitor /{print $2}'
    end
end

function __kitsune_profiles
    if type -q kitsune
        kitsune profiles list all 2>/dev/null
    else if test -d ./config/profiles
        for f in ./config/profiles/*.profile
            basename $f .profile
        end
    end
end

complete -c kitsune -f
complete -c kitsune -n '__fish_use_subcommand' -a 'install start stop restart status doctor run logs layer-status config visual style mode backend output-target spectrum-mode group-file group runtime rotate rotation profiles test-load profile-edit tune dynamic-color color-poll colorwatch postfx particles particles-look particles-preset monitors monitor monitor-fallback autostart instances instance-status clean reset benchmark help'

complete -c kitsune -n '__fish_seen_subcommand_from start' -a '(__kitsune_monitors)'
complete -c kitsune -n '__fish_seen_subcommand_from start' -l monitor -a '(__kitsune_monitors)'
complete -c kitsune -n '__fish_seen_subcommand_from start' -l profile -a '(__kitsune_profiles)'
complete -c kitsune -n '__fish_seen_subcommand_from start' -l profiles
complete -c kitsune -n '__fish_seen_subcommand_from start' -l target -a 'mpvpaper layer-shell'
complete -c kitsune -n '__fish_seen_subcommand_from start' -l mode -a 'bars ring'

complete -c kitsune -n '__fish_seen_subcommand_from stop' -a '(__kitsune_monitors)'
complete -c kitsune -n '__fish_seen_subcommand_from stop' -l monitor -a '(__kitsune_monitors)'

complete -c kitsune -n '__fish_seen_subcommand_from logs' -a 'renderer cava mpvpaper layer colorwatch monitorwatch all'
complete -c kitsune -n '__fish_seen_subcommand_from logs' -l lines
complete -c kitsune -n '__fish_seen_subcommand_from logs' -s f
complete -c kitsune -n '__fish_seen_subcommand_from logs' -l follow
complete -c kitsune -n '__fish_seen_subcommand_from logs' -l all-instances

complete -c kitsune -n '__fish_seen_subcommand_from status' -l all-instances
complete -c kitsune -n '__fish_seen_subcommand_from doctor' -l all-instances
complete -c kitsune -n '__fish_seen_subcommand_from doctor' -l fix

complete -c kitsune -n '__fish_seen_subcommand_from backend' -a 'cpu gpu'
complete -c kitsune -n '__fish_seen_subcommand_from output-target' -a 'mpvpaper layer-shell'
complete -c kitsune -n '__fish_seen_subcommand_from spectrum-mode' -a 'single group'
complete -c kitsune -n '__fish_seen_subcommand_from runtime' -a 'standard test'
complete -c kitsune -n '__fish_seen_subcommand_from rotate' -a '0 1 next prev shuffle seed --apply'
complete -c kitsune -n '__fish_seen_subcommand_from profiles' -a 'list show set-list set-static rotate clone set bars ring all on off'
complete -c kitsune -n '__fish_seen_subcommand_from profiles' -a '(__kitsune_profiles)'
complete -c kitsune -n '__fish_seen_subcommand_from test-load' -a '(__kitsune_profiles)'

complete -c kitsune -n '__fish_seen_subcommand_from group' -a 'validate list-layers add-layer update-layer remove-layer'
complete -c kitsune -n '__fish_seen_subcommand_from monitors' -a 'list'
complete -c kitsune -n '__fish_seen_subcommand_from monitor' -a 'set (__kitsune_monitors)'
complete -c kitsune -n '__fish_seen_subcommand_from monitor-fallback' -a '0 1'
complete -c kitsune -n '__fish_seen_subcommand_from autostart' -a 'enable disable status list'
complete -c kitsune -n '__fish_seen_subcommand_from autostart' -l monitor -a '(__kitsune_monitors)'
complete -c kitsune -n '__fish_seen_subcommand_from instances' -a 'list status (__kitsune_monitors)'
complete -c kitsune -n '__fish_seen_subcommand_from instance-status' -a '(__kitsune_monitors)'
complete -c kitsune -n '__fish_seen_subcommand_from clean' -l force
complete -c kitsune -n '__fish_seen_subcommand_from reset' -l restart
complete -c kitsune -n '__fish_seen_subcommand_from help' -a 'restart logs layer-status doctor config rotate instances'
