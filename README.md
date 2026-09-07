# Hyprland Dotfiles

My personal Arch dotfiles: Hyprland setup with systemd user service
management, plus shell and editor-adjacent configs.

## Architecture

Services are managed via `systemd --user`:

- `hyprland-session.target` — session lifecycle target
- `hyprland.conf` runs `exec-once = ~/.config/hypr/scripts/init.sh` on startup
- `init.sh` imports Wayland env vars into systemd, then starts `hyprland-session.target`
- All daemons (`waybar`, `swaync`, `hyprpaper`, `hypridle`, `fcitx5-daemon`, `copyq`) bind to this target via `BindsTo=` / `PartOf=`

Bluetooth is **on-demand** to save battery: `bluetooth.service` and the radio are off when idle. The waybar `bt-toggle` custom module shows the current state, and clicking it starts `bluetooth.service` (via the passwordless sudoers rule that `install.sh` sets up), powers the adapter on, and opens `blueman-manager`. The tray applet (`blueman-applet`) is not used.

This means when Hyprland exits, all services are cleaned up automatically.

## Installation

Tested on Arch / CachyOS and Debian (testing).

```bash
git clone https://github.com/pjh456/hyprland-conf ~/dotfiles
~/dotfiles/install.sh --packages
```

What `install.sh` does:

- `--packages`: installs required packages — official repos via `pacman`,
  AUR via your `yay`/`paru`/`buttercup` if one is present. Lists live in
  [`packages/`](packages/) and are validated by CI against the live Arch
  databases.
- Backs up any existing dotfiles to `~/.dotfiles-backup-<timestamp>/`, then
  symlinks everything from `etc/` into `$HOME`.
- Enables the session services (`systemctl --user`), installs uv tools
  (`hyprconf2lua`, `ruff`, `zhihu-tui`), and writes the NOPASSWD sudoers
  rule for on-demand bluetooth.

Before the first run, make sure these `pass` entries exist (`.bashrc` reads
them at shell startup): `snyk/token`, `huggingface/token`.

Flags: `--no-systemd`, `--no-sudo`, `--no-uv` skip the respective steps.
The script is idempotent — re-running it is safe.

### Debian notes

On Debian, `--packages` installs [`packages/debian.txt`](packages/debian.txt)
via `apt` and prints the packages that need a manual build. Debian stable
(trixie) predates hyprland in main — use testing, or stable with
trixie-backports. Two specifics:

- `tlp` conflicts with `power-profiles-daemon`. The `power-mode` waybar
  module needs the `net.hadess.PowerProfiles` D-Bus interface, which tlp
  1.10 provides built-in — enable it with `TLP_PD_ENABLE=1` in
  `/etc/tlp.conf`.
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

On first start, `init.sh` will stop and restart `hyprland-session.target` to pick up all services. On subsequent starts it only starts individual services that aren't running.

## Day-to-day

Every dotfile in `$HOME` is a symlink into `etc/`, so editing a live file
(`vim ~/.bashrc`) edits the repo directly — `git diff` / `git commit` works
as usual.

## Rollback

```bash
~/dotfiles/install.sh --restore            # from the newest backup
~/dotfiles/install.sh --restore <dir>      # or a specific ~/.dotfiles-backup-*/
```

Removes the symlinks pointing into the repo and restores the backed-up
originals. Not rolled back: systemd service enabling, uv tools, the
sudoers rule.

## Keybindings

| Key                   | Action                                           |
| --------------------- | ------------------------------------------------ |
| `Super + Return`      | Open foot terminal                               |
| `Super + Q`           | Close window                                     |
| `Super + Space`       | Rofi application launcher                        |
| `Super + L`           | Power menu (shutdown/reboot/lock/suspend/logout) |
| `Alt + Tab`           | Hyprswitch window switcher                       |
| `Super + V`           | Clipboard history (cliphist + rofi)              |
| `Super + T`           | Toggle floating                                  |
| `Super + N`           | Toggle notification center (swaync)              |
| `Super + F5`          | Reload Hyprland + Waybar                         |
| `Ctrl + Alt + A`      | Screenshot region (grim + slurp)                 |
| `Super + F`           | Fullscreen                                       |
| Waybar Bluetooth icon | Click: start `bluetooth.service` + power on adapter, then open `blueman-manager` (no-op if already on) |
| `Super + 1-5`         | Switch workspace                                 |
| `Super + Shift + 1-5` | Move window to workspace                         |
| `Super + R`           | Resize mode (arrow keys to resize)               |

## File Structure

```
├── install.sh                 # Deploy / restore entrypoint
├── packages/
│   ├── arch-official.txt      # pacman list (validated by CI)
│   ├── arch-aur.txt           # AUR list (yay/paru/buttercup)
│   ├── debian.txt             # apt list (validated by CI)
│   └── debian-manual.txt      # Debian manual-build list
├── etc/                       # 1:1 mirror of $HOME, symlinked in place
│   ├── .bashrc                # Aliases, starship, fzf, thefuck
│   ├── .bash_profile
│   ├── .gitconfig
│   ├── .config/
│   │   ├── hypr/              # hyprland.conf, hypridle, hyprlock, hyprpaper, scripts/init.sh
│   │   ├── waybar/            # Status bar config & style
│   │   ├── rofi/               # Launcher config
│   │   ├── swaync/             # Notification center config & style
│   │   ├── fcitx5/             # Input method config
│   │   ├── foot/               # Terminal config
│   │   └── systemd/user/       # Session services (*.wants/ not tracked)
│   └── .local/bin/            # powermenu, bt-toggle, power-mode, temperature,
│                             # weather, clipmenu, setwp, waybar-reload,
│                             # mpv-profile-{watch,switch}, on-battery, ...
└── .github/workflows/ci.yml   # shellcheck, JSON check, package resolution, dry-run
```
