#!/bin/dash

# GTK/Qt 主题：GTK4 与 libadwaita 应用（含 portal 的文件选择器）不看
# gtk-theme-name，只认 color-scheme，不设就会退回刺眼的 Adwaita 亮色。
gsettings set org.gnome.desktop.interface gtk-theme catppuccin-mocha-blue-standard+default
gsettings set org.gnome.desktop.interface icon-theme Papirus-Dark
gsettings set org.gnome.desktop.interface cursor-theme catppuccin-mocha-blue-cursors
gsettings set org.gnome.desktop.interface cursor-size 24
gsettings set org.gnome.desktop.interface font-name 'JetBrainsMono Nerd Font 11'
gsettings set org.gnome.desktop.interface color-scheme prefer-dark

# 由 systemd / D-Bus 拉起的进程（portal、polkit 认证框、托盘应用）不会继承
# Hyprland 的 hl.env，必须把这些变量显式灌进 systemd 和 D-Bus 激活环境，
# 否则它们的弹窗会以默认亮色主题启动。
theme_env="GTK_THEME XCURSOR_THEME XCURSOR_SIZE QT_QPA_PLATFORMTHEME QT_QPA_PLATFORM"
systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE XDG_RUNTIME_DIR $theme_env
dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE XDG_RUNTIME_DIR DISPLAY $theme_env

if [ ! -f /tmp/hyprland-init-done ]; then
    touch /tmp/hyprland-init-done
    systemctl --user stop hyprland-session.target
    systemctl --user start hyprland-session.target
else
    systemctl --user reset-failed
    for unit in waybar.service swaync.service hyprpaper.service hypridle.service fcitx5-daemon.service hyprpolkitagent.service; do
        systemctl --user is-active --quiet "$unit" 2>/dev/null || systemctl --user start "$unit" 2>/dev/null
    done
fi

# 已经跑着的 portal 进程要重启才会拿到上面的主题环境。
systemctl --user restart xdg-desktop-portal-gtk.service 2>/dev/null
systemctl --user restart xdg-desktop-portal-hyprland.service 2>/dev/null
systemctl --user restart xdg-desktop-portal.service 2>/dev/null

# Restore the last wallpaper picked with setwp (also used by hyprlock).
wallpaper="${XDG_CACHE_HOME:-$HOME/.cache}/current_wallpaper"
if [ -f "$wallpaper" ]; then
    "$HOME/.local/bin/setwp" "$wallpaper" >/dev/null 2>&1
fi

pgrep -x wl-paste >/dev/null || wl-paste --type text --watch cliphist store 2>/dev/null &

pgrep -x hyprswitch >/dev/null || nohup hyprswitch init > /dev/null 2>&1 &
