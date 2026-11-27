# Hyprland Dotfiles

English | [中文文档](docs/README-zh.md)

My personal Arch dotfiles: Hyprland setup with systemd user service
management, plus shell and editor-adjacent configs.

![screenshot](docs/screenshot.png)

## Architecture

Services are managed via `systemd --user`:

- `hyprland-session.target` — session lifecycle target
- `hyprland.lua` runs `~/.config/hypr/scripts/init.sh` on startup (via the `hyprland.start` event)
- `init.sh` imports Wayland env vars into systemd, then starts `hyprland-session.target`
- All daemons (`waybar`, `swaync`, `hyprpaper`, `hypridle`, `fcitx5-daemon`) bind to this target via `BindsTo=` / `PartOf=`

Bluetooth is **on-demand** to save battery: `bluetooth.service` and the radio are off when idle. The waybar `bt-toggle` custom module shows the current state, and clicking it starts `bluetooth.service` (via the passwordless sudoers rule that `install.sh` sets up), powers the adapter on, and opens `blueman-manager`. The tray applet (`blueman-applet`) is not used.

This means when Hyprland exits, all services are cleaned up automatically.

### Wayland-native policy

Everything that can run on Wayland does, so dialogs and popups are sized and
positioned by the compositor instead of XWayland:

- Qt: `QT_QPA_PLATFORM=wayland;xcb`
- GTK3: `GDK_BACKEND=wayland` (Thunar, pavucontrol, zathura, portal dialogs);
  also imported into the systemd/D-Bus activation environment by `init.sh`
- Firefox: `MOZ_ENABLE_WAYLAND=1`; Chromium: `chrome-flags.conf`;
  QQ: `qq-flags.conf`; other Electron apps: `ELECTRON_OZONE_PLATFORM_HINT=auto`
- Steam/Proton, Wine and a few SDL games intentionally stay on XWayland.

## Installation

Tested on Arch / CachyOS and Debian (testing).

```bash
git clone https://github.com/pjh456/dotfiles ~/dotfiles
~/dotfiles/install.sh --packages
```

By default `install.sh`:

- backs up any existing dotfiles to `~/.dotfiles-backup-<timestamp>/`, then
  copies everything from `etc/` into `$HOME` as regular files;
- enables the session services whose binaries are installed (`systemctl --user`);
- writes the NOPASSWD sudoers rule for on-demand bluetooth (and remembers it,
  so re-runs don't ask for a password);
- comments out the volume/brightness commands in `/etc/acpi/handler.sh` (only
  if `acpid` is installed), so acpid and Hyprland's media-key binds don't run
  the same key twice.

Options:

| Option | Effect |
|---|---|
| `--packages` | Also install the required packages — official repos via `pacman` (Arch) or `apt` (Debian), AUR via your `yay`/`paru`/`buttercup` if one is present; on Debian the manual-build packages are printed instead. Lists live in [`packages/`](packages/), validated by CI against the live databases. |
| `--no-<pkg>` | With `--packages`: do not install package `<pkg>`. The name must exist in the detected distro's package lists — unknown names are rejected. |
| `--restore [DIR]` | Undo a deployment: remove the deployed files (tracked in `~/.dotfiles-deployed`) and restore the backed-up originals from `DIR` (default: newest `~/.dotfiles-backup-*`). Systemd enabling, the sudoers rule and the acpid patch are not rolled back. |
| `--no-systemd` | Skip `daemon-reload` and enabling the session services. |
| `--no-sudo` | Skip the system-level tweaks: the on-demand bluetooth sudoers rule and the acpid media-key patch. |
| `-h`, `--help` | Show usage. |

Companion packages are excluded together: `--no-tlp` also excludes
`tlp-pd`.

The script is idempotent — re-running it is safe.

Before the first run, make sure these `pass` entries exist (`.bashrc` reads
them at shell startup): `snyk/token`, `huggingface/token`.

### Debian notes

On Debian, `--packages` installs [`packages/debian.txt`](packages/debian.txt)
via `apt` and prints the packages that need a manual build. Debian stable
(trixie) predates hyprland in main — use testing, or stable with
trixie-backports. Two specifics:

- `tlp` conflicts with `power-profiles-daemon`. The waybar
  `power-profiles-daemon` module (and the `power-mode` script) need the
  `net.hadess.PowerProfiles` D-Bus interface, which tlp 1.10 provides
  built-in — enable it with `TLP_PD_ENABLE=1` in `/etc/tlp.conf`.
- Arch's `fcitx5-gtk` is `fcitx5-frontend-gtk3` / `fcitx5-frontend-gtk4`
  on Debian.

#### Manual builds (Debian)

No Debian package exists for these; build from source:

| Package                        | Source                                                        |
| ------------------------------ | ------------------------------------------------------------- |
| hypridle                       | https://github.com/hyprwm/hypridle                            |
| hyprlock                        | https://github.com/hyprwm/hyprlock                            |
| hyprpaper                       | https://github.com/hyprwm/hyprpaper                           |
| hyprswitch                      | https://github.com/hyprwm/hyprswitch                          |
| swaync                          | https://github.com/elkowar/swaync                             |
| fonts-jetbrains-mono-nerd       | https://github.com/nerd-fonts/nerd-fonts (install script)     |

Then start Hyprland:

```bash
start-hyprland
```

Every start restarts `hyprland-session.target`, so the daemons re-attach to the new compositor's Wayland socket. After a crash-and-restart a service can still look `active` while its connection is already dead — `fcitx5` in particular then loses the input method silently, so starting only the inactive units is not enough. `init.sh` also exits early when the Hyprland instance that launched it is already gone (a fast restart can leave it orphaned).

## Day-to-day

The repo's `etc/` is the source of truth; `$HOME` holds regular copies.
Edit files in the repo, then re-run `~/dotfiles/install.sh` to apply —
up-to-date files are left alone, changed ones are re-copied. Edits made
directly in `$HOME` do not reach the repo; copy them back into `etc/`
before committing if you want to keep them.

## Rollback

```bash
~/dotfiles/install.sh --restore            # from the newest backup
~/dotfiles/install.sh --restore <dir>      # or a specific ~/.dotfiles-backup-*/
```

Removes the deployed files (tracked in `~/.dotfiles-deployed`) and
restores the backed-up originals. Not rolled back: systemd service
enabling, the sudoers rule.

## Keybindings

| Key                   | Action                                           |
| --------------------- | ------------------------------------------------ |
| `Super + Return`      | Open foot terminal                               |
| `Super + Q`           | Close window                                     |
| `Super + Space`       | Rofi application launcher                        |
| `Super + .`           | Color emoji picker (inserts into the focused window; copies to the clipboard when `wtype` is not installed) |
| `Super + L`           | Power menu (shutdown/reboot/lock/suspend/logout) |
| `Alt + Tab`           | Hyprswitch window switcher                       |
| `Super + V`           | Clipboard history (cliphist + rofi)              |
| `Super + T`           | Toggle floating                                  |
| `Super + N`           | Toggle notification center (swaync)              |
| `Super + F5`          | Reload Hyprland + restart Waybar                 |
| `Ctrl + Alt + A`      | Screenshot region (grim + slurp)                 |
| `Super + F`           | Fullscreen                                       |
| Waybar Bluetooth icon | Click: start `bluetooth.service` + power on adapter, then open `blueman-manager` (no-op if already on) |
| `Super + 1-5`         | Switch workspace                                 |
| `Super + Shift + 1-5` | Move window to workspace                         |
| `Super + R`           | Resize mode (arrow keys to resize)               |
| `XF86Audio*` / `XF86MonBrightness*` | Volume / brightness OSD popup (swaync) |

## Helper scripts

All live in `~/.local/bin/` — the ones you run yourself, and the ones the
configs invoke.

### Run manually

| Script | What it does |
|---|---|
| `setwp <image>` | Set the wallpaper from any file: `setwp ~/Pictures/wall.jpg` (goes through `hyprpaper`, so the live compositor keeps it) |
| `unsetwp` | Back to the solid-black wallpaper (keeps hyprpaper alive) |
| `powermenu` | Rofi power menu — also the `Super + L` binding; a second press closes it |
| `clipmenu` | Scripted clipboard history: the `Super + V` pipeline, plus a "copied" notification and `--paste-once` |
| `rofi-toggle <args>` | Toggle any rofi menu (`rofi-toggle -show drun`): opens it, or closes it when already open |
| `emoji` | Color emoji picker — the `Super + .` binding. Types the emoji into the focused window with `wtype`, otherwise leaves it on the clipboard |
| `power-mode [status\|next]` | Power profile from the CLI (`power-mode next`); the bar uses waybar's built-in `power-profiles-daemon` module |
| `waybar-reload` | Restart waybar after editing its config (`systemctl --user restart waybar.service` when it runs under systemd); `Super + F5` calls it together with a Hyprland reload |
| `mpv-profile-switch [profile]` | Force-sync the mpv profile now — links `mpv.conf.{PRF,BAL,SAV}` → `mpv.conf`. With no argument it asks TLP; `mpv-profile-watch` passes the D-Bus value straight in |
| `power-profile-daemon` | Run in a terminal: watches AC plug events (inotify) and switches `performance` / `power-saver` on the D-Bus power-profiles interface |
| `on-battery` | Exits 0 only while on battery — for one-liners: `on-battery && mpv-profile-switch` |

### Invoked by the configs (not for manual use)

| Script | Invoked by |
|---|---|
| `bt-toggle [status\|toggle]` | waybar bluetooth module (5s poll + click) |
| `temperature` | waybar temperature module |
| `weather` | waybar weather module (30 min poll); also caches the 7-day forecast |
| `weather-forecast` | click on the waybar weather module; renders the cached forecast with rofi |
| `mpv-profile-watch` | user service; re-runs `mpv-profile-switch` when `net.hadess.PowerProfiles.ActiveProfile` changes (reads the property every 10s: ~2ms, not `tlp-stat`'s ~90ms) |
| `osd-ctl volume up\|down\|mute`, `osd-ctl brightness up\|down` | Hyprland media keys |

## File Structure

```
├── install.sh                 # Deploy / restore entrypoint
├── docs/
│   └── README-zh.md           # Chinese documentation
├── packages/
│   ├── arch-official.txt      # pacman list (validated by CI)
│   ├── arch-aur.txt           # AUR list (yay/paru/buttercup)
│   ├── debian.txt             # apt list (validated by CI)
│   └── debian-manual.txt      # Debian manual-build list
├── etc/                       # 1:1 mirror of $HOME (copied into place by install.sh)
│   ├── .bashrc                # Aliases, starship, fzf, thefuck
│   ├── .bash_profile
│   ├── .gitconfig
│   ├── .config/
│   │   ├── chrome-flags.conf  # Chromium/Electron: wayland native + Intel VA-API
│   │   ├── qq-flags.conf      # QQ: native Wayland (avoids XWayland dialog sizing)
│   │   ├── starship.toml      # Catppuccin two-line prompt
│   │   ├── fastfetch/         # Grouped system info card
│   │   ├── btop/              # Catppuccin theme, transparent background
│   │   ├── hypr/              # hyprland.lua (+anim/keys/notify), hypridle, hyprlock, hyprpaper, scripts/init.sh
│   │   ├── waybar/            # Status bar config & style
│   │   ├── rofi/               # Launcher config
│   │   ├── swaync/             # Notification center config & style
│   │   ├── fcitx5/             # Input method config
│   │   ├── foot/               # Terminal config
│   │   └── systemd/user/       # Session services (*.wants/ not tracked)
│   ├── .local/bin/            # powermenu, bt-toggle, power-mode, temperature,
│   │                         # weather, weather-forecast, clipmenu, setwp, waybar-reload,
│   │                         # osd-ctl, mpv-profile-{watch,switch}, on-battery, ...
│   └── .local/share/
│       └── fcitx5/themes/     # Self-contained fcitx5 theme (svg + png)
└── .github/workflows/ci.yml   # shellcheck, JSON check, package resolution, dry-run
```
