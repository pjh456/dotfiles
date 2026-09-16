local notify = require("notify")

local mainMod = "SUPER"

hl.bind(mainMod .. " + Return", hl.dsp.exec_cmd("foot"))

hl.bind(mainMod .. " + Q", hl.dsp.window.close())

hl.bind(mainMod .. " + code:9", hl.dsp.exit())

hl.bind(mainMod .. " + Space", hl.dsp.exec_cmd("rofi -show drun"))

hl.bind(mainMod .. " + L", hl.dsp.exec_cmd("/home/pjh123/.local/bin/powermenu"))

hl.bind("ALT + Tab", hl.dsp.exec_cmd("hyprswitch simple"))

hl.bind(mainMod .. " + V", hl.dsp.exec_cmd("cliphist list| rofi -dmenu| cliphist decode| wl-copy"))

hl.bind(mainMod .. " + T", hl.dsp.window.float())

hl.bind(mainMod .. " + N", hl.dsp.exec_cmd("swaync-client -t -sw"))

hl.bind(mainMod .. " + F5", function()
	hl.exec_cmd("hyprctl reload && pkill -SIGUSR2 waybar")
	notify.ok("Config", "Reloaded")
end)

hl.bind("CTRL + ALT + A", hl.dsp.exec_cmd("grim -g \"$(slurp)\" - | tee >(wl-copy --type image/png) | cliphist store"))

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
