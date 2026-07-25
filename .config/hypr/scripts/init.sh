#!/bin/dash

gsettings set org.gnome.desktop.interface gtk-theme catppuccin-mocha-blue-standard+default
gsettings set org.gnome.desktop.interface icon-theme Papirus-Dark
gsettings set org.gnome.desktop.interface cursor-theme catppuccin-mocha-blue-cursors
gsettings set org.gnome.desktop.interface cursor-size 24
gsettings set org.gnome.desktop.interface font-name 'JetBrainsMono Nerd Font 11'

systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE XDG_RUNTIME_DIR

if [ ! -f /tmp/hyprland-init-done ]; then
    touch /tmp/hyprland-init-done
    systemctl --user stop hyprland-session.target
    systemctl --user start hyprland-session.target
else
    systemctl --user reset-failed
    for unit in waybar.service swaync.service hyprpaper.service hypridle.service fcitx5-daemon.service; do
        systemctl --user is-active --quiet "$unit" 2>/dev/null || systemctl --user start "$unit" 2>/dev/null
    done
fi

wl-paste --type text --watch cliphist store 2>/dev/null &

nohup hyprswitch init > /dev/null 2>&1 &
