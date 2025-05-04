# displayconfig-mutter

`xrandr` like app for Gnome DE in Wayland mode. Allows modifying most parameters that are exposed in "Displays" settings.

Go to [installation instructions](#installation).

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
      --refresh-rate <REFRESH_RATE>  New monitor refresh rate. This is selected on a best effort basis. e.g. if you select 60Hz, while monitor only supports 59.98Hz, it will be selected instead
      --max-refresh-rate             Automatically select highest refresh rate for selected resolution
      --vrr <VRR>                    Controls variable refresh rate [possible values: true, false]
      --scaling <SCALING>            UI Scaling, as precentage, e.g. 100, 150, 200. This is selected based on a closest available scaling with a rounding step of 25%. e.g. if you select 125, while selected resolution only allows for either 124% or 149% - first one will be selected
      --hdr <HDR>                    Controls high dynamic range color mode [possible values: true, false]
  -h, --help                         Print help
```

## Installation

### NixOS / Nix
```
nix run github:eaglesemanation/displayconfig-mutter -- help
```

### Arch Linux
Available through [AUR](https://aur.archlinux.org/packages/displayconfig-mutter). You can clone it and build it manually, or use one of many AUR helpers, such as [yay](https://github.com/Jguer/yay)
```
yay -S displayconfig-mutter
```

### Fedora Linux
Available through [COPR](https://copr.fedorainfracloud.org/coprs/eaglesemanation/displayconfig-mutter/).
```
sudo dnf copr enable eaglesemanation/displayconfig-mutter
sudo dnf install displayconfig-mutter
```

### Ubuntu / Linux Mint
Available through [Launchpad PPA](https://launchpad.net/~eaglesemanation/+archive/ubuntu/displayconfig-mutter).
```
sudo add-apt-repository ppa:eaglesemanation/displayconfig-mutter
sudo apt update
sudo apt install displayconfig-mutter
```

### Others
Releases page contains pre-built binaries for x86_64 (Intel / AMD) and aarch64 (ARM) processors. You can install them on most distros by running this:
```
curl "https://github.com/eaglesemanation/displayconfig-mutter/releases/latest/download/displayconfig-mutter-$(uname -m)" -L -o displayconfig-mutter \
  && sudo install -Dm0755 displayconfig-mutter /usr/local/bin/displayconfig-mutter \
  && rm displayconfig-mutter
```
