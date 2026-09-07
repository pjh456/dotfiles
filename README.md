# Hyprland Dotfiles

My personal Hyprland configuration with systemd user service management.

## Architecture

Services are managed via `systemd --user`:

- `hyprland-session.target` — session lifecycle target
- `hyprland.conf` runs `exec-once = ~/.config/hypr/scripts/init.sh` on startup
- `init.sh` imports Wayland env vars into systemd, then starts `hyprland-session.target`
- All daemons (`waybar`, `swaync`, `hyprpaper`, `hypridle`, `fcitx5-daemon`, `copyq`) bind to this target via `BindsTo=` / `PartOf=`

Bluetooth is **on-demand** to save battery: `bluetooth.service` and the radio are off when idle. The waybar `bt-toggle` custom module shows the current state, and clicking it starts `bluetooth.service` (via passwordless sudo, see installation), powers the adapter on, and opens `blueman-manager`. The tray applet (`blueman-applet`) is not used.

This means when Hyprland exits, all services are cleaned up automatically.

## Installation

### 1. Install packages

```bash
# Core
sudo pacman -S hyprland waybar foot rofi swaync
sudo pacman -S hypridle hyprlock hyprpaper
sudo pacman -S fcitx5 fcitx5-chinese-addons fcitx5-gtk
sudo pacman -S cliphist wl-clipboard grim slurp
sudo pacman -S copyq blueman
sudo pacman -S lm_sensors tlp-pd
sudo pacman -S ttf-jetbrains-mono-nerd noto-fonts-cjk
sudo pacman -S papirus-icon-theme

# Shell (bashrc + mpv/tlp scripts)
sudo pacman -S tlp pass starship fzf thefuck

# AUR (use your preferred AUR helper, e.g. yay)
yay -S hyprswitch iwgtk catppuccin-mocha-gtk-themes catppuccin-mocha-cursors
```

### 2. Clone and deploy

```bash
git clone https://github.com/pjh456/hyprland-conf ~/hyprland-dotfiles
cd ~/hyprland-dotfiles

# Back up your existing configs first!
cp -r ~/.config/hypr ~/.config/hypr.bak
cp -r ~/.config/waybar ~/.config/waybar.bak
# ... etc

# Deploy
cp -r .config/* ~/.config/
cp -r .local/bin/* ~/.local/bin/
```

### 3. Enable systemd user services

```bash
# Reload systemd user daemon
systemctl --user daemon-reload

# Passwordless start/stop of bluetooth.service so the waybar click works without a
# sudo prompt (one-time, requires sudo; adjust the username)
echo 'pjh123 ALL=(ALL) NOPASSWD: /usr/bin/systemctl start bluetooth.service, /usr/bin/systemctl stop bluetooth.service' \
  | sudo tee /etc/sudoers.d/bluetooth-ondemand

# Enable all session services
for s in waybar swaync hyprpaper hypridle fcitx5-daemon copyq; do
  systemctl --user enable "$s"
done
```

### 4. Start Hyprland

```bash
start-hyprland
```

On first start, `init.sh` will stop and restart `hyprland-session.target` to pick up all services. On subsequent starts it only starts individual services that aren't running.

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
.config/
├── hypr/
│   ├── hyprland.conf      # Main Hyprland config
│   ├── hypridle.conf       # Idle management
│   ├── hyprlock.conf       # Lock screen
│   ├── hyprpaper.conf      # Wallpaper
│   └── scripts/
│       └── init.sh         # Session init (systemd service orchestration)
├── systemd/user/
│   ├── hyprland-session.target      # Session lifecycle target
│   ├── waybar.service
│   ├── swaync.service
│   ├── hyprpaper.service
│   ├── hypridle.service
│   ├── fcitx5-daemon.service
│   ├── copyq.service
│   └── mpv-profile-watch.service
│   └── *.target.wants/             # NOT tracked — run systemctl --user enable to create
├── waybar/                 # Status bar config & style
├── rofi/                   # Launcher config
├── swaync/                 # Notification center config & style
├── fcitx5/                 # Input method config
└── foot/                   # Terminal config
.local/bin/
├── powermenu              # Power menu script
├── bt-toggle              # On-demand bluetooth toggle (waybar module)
├── power-mode             # Power-profiles-daemon mode switcher (waybar module)
├── temperature            # CPU temperature (waybar module, lm-sensors)
├── weather                # wttr.in one-line weather (waybar module)
└── mpv-profile-watch      # TLP profile -> mpv config watcher (systemd user service)
```
