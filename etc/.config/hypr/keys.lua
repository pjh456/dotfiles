local notify = require("notify")

local mainMod = "SUPER"

hl.bind(mainMod .. " + Return", hl.dsp.exec_cmd("foot"))

hl.bind(mainMod .. " + Q", hl.dsp.window.close())

hl.bind(mainMod .. " + code:9", hl.dsp.exit())

hl.bind(mainMod .. " + Space", hl.dsp.exec_cmd("~/.local/bin/rofi-toggle -show drun"))

-- Color emoji picker (Windows Win+.): inserts into the focused window when
-- wtype is installed, otherwise copies the emoji and asks for Ctrl+V. QQ
-- ignores synthesized emoji keys, so it gets clipboard + Ctrl+V instead.
hl.bind(mainMod .. " + period", function()
	local w = hl.get_active_window()
	local app = w and w.class or ""
	local paste = app == "QQ" or app == "qq" or app == "linuxqq" or app == "com.tencent.qq"
	hl.exec_cmd("~/.local/bin/emoji" .. (paste and " paste" or ""))
end)

hl.bind(mainMod .. " + L", hl.dsp.exec_cmd("/home/pjh123/.local/bin/powermenu"))

hl.bind("ALT + Tab", hl.dsp.exec_cmd("hyprswitch simple"))

hl.bind(mainMod .. " + V", hl.dsp.exec_cmd("~/.local/bin/clipmenu"))

-- Weather: same popup as clicking the top-left waybar weather module.
-- Waybar tooltips cannot be triggered from a keybind, so this opens the
-- cached 7-day rofi forecast instead.
hl.bind(mainMod .. " + W", hl.dsp.exec_cmd("~/.local/bin/weather-forecast"))

hl.bind(mainMod .. " + N", hl.dsp.exec_cmd("swaync-client -t -sw"))

hl.bind(mainMod .. " + F5", function()
	hl.exec_cmd("hyprctl reload && ~/.local/bin/waybar-reload")
	notify.ok("Config", "Reloaded")
end)

hl.bind("CTRL + ALT + A", hl.dsp.exec_cmd("grim -g \"$(slurp -b 1e1e2e66 -c 89b4faff -s cba6f71a -w 2)\" - | tee >(wl-copy --type image/png) | cliphist store"))

-- Media keys drive the same OSD the waybar widgets use.
hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("~/.local/bin/osd-ctl volume up"), { repeating = true })
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("~/.local/bin/osd-ctl volume down"), { repeating = true })
hl.bind("XF86AudioMute", hl.dsp.exec_cmd("~/.local/bin/osd-ctl volume mute"))
hl.bind("XF86MonBrightnessUp", hl.dsp.exec_cmd("~/.local/bin/osd-ctl brightness up"), { repeating = true })
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd("~/.local/bin/osd-ctl brightness down"), { repeating = true })

hl.bind(mainMod .. " + F", hl.dsp.window.fullscreen())

local dirs = { "left", "right", "up", "down" }
for _, d in ipairs(dirs) do
	hl.bind(mainMod .. " + " .. d, hl.dsp.focus({ direction = d }))
end

for i = 1, 5 do
	hl.bind(mainMod .. " + " .. i, hl.dsp.focus({ workspace = i }))
	hl.bind(mainMod .. " + SHIFT + " .. i, hl.dsp.window.move({ workspace = i }))
end

hl.bind(mainMod .. " + R", hl.dsp.submap("resize"))

hl.define_submap("resize", function()
	hl.bind("left",   hl.dsp.window.resize({ x = -30, y = 0, relative = true }), { repeating = true })
	hl.bind("right",  hl.dsp.window.resize({ x = 30, y = 0, relative = true }), { repeating = true })
	hl.bind("up",     hl.dsp.window.resize({ x = 0, y = -30, relative = true }), { repeating = true })
	hl.bind("down",   hl.dsp.window.resize({ x = 0, y = 30, relative = true }), { repeating = true })
	hl.bind(mainMod .. " + R", hl.dsp.submap("reset"))
	hl.bind("escape", hl.dsp.submap("reset"))
end)

hl.on("keybinds.submap", function(name)
	if name == "resize" then
		notify.info("Resize", "Mode on")
	elseif name == "" then
		notify.info("Resize", "Mode off")
	end
end)
