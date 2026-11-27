hl.curve("easeOutQuint",   { type = "bezier", points = { {0.23, 1}, {0.32, 1} } })
hl.curve("easeInOutCubic", { type = "bezier", points = { {0.65, 0.05}, {0.36, 1} } })
hl.curve("linear",         { type = "bezier", points = { {0, 0}, {1, 1} } })
hl.curve("almostLinear",   { type = "bezier", points = { {0.5, 0.5}, {0.75, 1} } })
hl.curve("quick",          { type = "bezier", points = { {0.15, 0}, {0.1, 1} } })
hl.curve("easy",           { type = "spring", mass = 1, stiffness = 71.2633, dampening = 15.8273644 })

hl.config({
	animations = {
		enabled = true,
	},
})

hl.animation({ leaf = "windows",       enabled = true, speed = 4, bezier = "easeOutQuint", style = "popin 85%" })
hl.animation({ leaf = "windowsIn",     enabled = true, speed = 4, bezier = "easeOutQuint", style = "popin 80%" })
hl.animation({ leaf = "windowsOut",    enabled = true, speed = 3, bezier = "easeOutQuint", style = "popin 88%" })
hl.animation({ leaf = "windowsMove",   enabled = true, speed = 4, bezier = "easeOutQuint" })
hl.animation({ leaf = "fade",          enabled = true, speed = 4, bezier = "easeOutQuint" })
hl.animation({ leaf = "fadeIn",        enabled = true, speed = 4, bezier = "easeOutQuint" })
hl.animation({ leaf = "fadeOut",       enabled = true, speed = 4, bezier = "easeOutQuint" })
hl.animation({ leaf = "border",        enabled = true, speed = 4, bezier = "easeOutQuint" })
hl.animation({ leaf = "layers",        enabled = true, speed = 3, bezier = "easeOutQuint" })
hl.animation({ leaf = "layersIn",      enabled = true, speed = 3, bezier = "easeOutQuint", style = "fade" })
hl.animation({ leaf = "layersOut",     enabled = true, speed = 3, bezier = "easeOutQuint", style = "fade" })
hl.animation({ leaf = "workspaces",    enabled = true, speed = 4, bezier = "easeOutQuint", style = "slide" })
hl.animation({ leaf = "workspacesIn",  enabled = true, speed = 4, bezier = "easeOutQuint", style = "slide" })
hl.animation({ leaf = "workspacesOut", enabled = true, speed = 4, bezier = "easeOutQuint", style = "slide" })
