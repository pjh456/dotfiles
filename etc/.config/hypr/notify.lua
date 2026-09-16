local M = {}

local SEND = "notify-send"

function M.info(title, msg, timeout)
	hl.exec_cmd(string.format("%s -i dialog-information '%s' '%s'", SEND, title, msg))
end

function M.ok(title, msg)
	hl.exec_cmd(string.format("%s -i dialog-ok '%s' '%s'", SEND, title, msg))
end

function M.err(title, msg)
	hl.exec_cmd(string.format("%s -i dialog-error '%s' '%s'", SEND, title, msg))
end

function M.warn(title, msg)
	hl.exec_cmd(string.format("%s -i dialog-warning '%s' '%s'", SEND, title, msg))
end

return M
