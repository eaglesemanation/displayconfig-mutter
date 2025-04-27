# displayconfig-mutter

`xrandr` like app for Gnome DE in Wayland mode. Allows modifying most parameters that are exposed in "Displays" settings.

```
# displayconfig-mutter --help
Usage: displayconfig-mutter <COMMAND>

Commands:
  list  List monitors
  set   Set config
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

```
# displayconfig-mutter list --help
List monitors

Usage: displayconfig-mutter list [OPTIONS]

Options:
  -c, --connector <CONNECTOR>  If specified - will list all available modes for a monitor with matching connector name
  -h, --help                   Print help
```

```
# displayconfig-mutter set --help
Set config

Usage: displayconfig-mutter set [OPTIONS] --connector <CONNECTOR>

Options:
  -c, --connector <CONNECTOR>        Name of monitor connector, e.g. DP-1, HDMI-2
  -p, --persistent                   Save config to the disk after applying it. Will prompt for user input to verify if it's correct
  -r, --resolution <RESOLUTION>      New resolution, e.g. 1920x1080, 3840x2160
      --max-resolution               Automatically select highest available refresh rate
      --refresh-rate <REFRESH_RATE>  Monitor refresh rate
      --max-refresh-rate             Automatically select highest refresh rate for selected resolution
      --vrr <VRR>                    Controls variable refresh rate [possible values: true, false]
      --scaling <SCALING>            UI Scaling, as precentage, e.g. 100, 150, 200
      --hdr <HDR>                    Controls high dynamic range color mode [possible values: true, false]
  -h, --help                         Print help
```

## Installation

### Nix

```
nix run github:eaglesemanation/displayconfig-mutter -- help
```
