local InfoMessage = require("ui/widget/infomessage")
local UIManager = require("ui/uimanager")
local _ = require("gettext")
local widgetcontainer = require("ui.widget.container.widgetcontainer")

local function shellQuote(value)
	return "'" .. value:gsub("'", "'\\''") .. "'"
end

local Weeb = widgetcontainer:extend({
	name = "weeb_central",
	is_doc_only = false,
})

function Weeb:init()
	self.ui.menu:registerToMainMenu(self)
end

function Weeb:addToMainMenu(menu_items)
	menu_items.weeb_central = {
		text = _("WeebCentral Downloader"),
		sorting_hint = "tools",
		sub_item_table = {
			{
				text = _("Jalankan core (dev)"),
				callback = function()
					self:runCore()
				end,
			},
		},
	}
end

function Weeb:runCore()
	local source = debug.getinfo(1, "S").source
	local plugin_directory = source:match("^@(.*/)")
	if not plugin_directory then
		self:showMessage(_("Tidak dapat menentukan lokasi plugin."))
		return
	end

	local binary = plugin_directory .. "bin/weebcentral_core_dev"
	local pipe = io.popen(shellQuote(binary) .. " 2>&1", "r")
	if not pipe then
		self:showMessage(_("Gagal menjalankan binary WeebCentral."))
		return
	end

	local output = pipe:read("*a") or ""
	local succeeded = pipe:close()
	if output == "" then
		output = _("Binary selesai tanpa output.")
	end

	if succeeded then
		self:showMessage(output)
	else
		self:showMessage(_("Binary gagal dijalankan:\n") .. output)
	end
end

function Weeb:showMessage(message)
	local popup = InfoMessage:new({
		text = message,
	})
	UIManager:show(popup)
end

return Weeb
