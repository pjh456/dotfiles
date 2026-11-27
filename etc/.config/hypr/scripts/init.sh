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
theme_env="GTK_THEME XCURSOR_THEME XCURSOR_SIZE QT_QPA_PLATFORMTHEME QT_QPA_PLATFORM GDK_BACKEND"
systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE XDG_RUNTIME_DIR $theme_env
dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE XDG_RUNTIME_DIR DISPLAY $theme_env

# Only touch the session if the compositor that spawned us is still there. On a
# fast crash/restart this script can be left orphaned: Hyprland is already gone,
# and any service started now would connect to a Wayland socket that no longer
# exists (fcitx5 then stays "active" with no input-method connection, so
# Ctrl+Space silently stops working in every Wayland app).
sig="${HYPRLAND_INSTANCE_SIGNATURE:-}"
rt="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
if [ -n "$sig" ] && [ ! -S "$rt/hypr/$sig/.socket.sock" ]; then
    exit 0
fi

# Always rebind the services to this instance. After a compositor restart the
# old waybar/fcitx5 may still be "active" from is-active's point of view even
# though their connection is dead, so starting only the inactive ones is not
# enough. Restarting the target propagates through PartOf=.
systemctl --user reset-failed
systemctl --user restart hyprland-session.target

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
