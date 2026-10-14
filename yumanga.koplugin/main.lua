local DataStorage = require("datastorage")
local widgetcontainer = require("ui.widget.container.widgetcontainer")
local UI = require("lib/yumanga/ui")

local YuManga = widgetcontainer:extend({
	name = "yumanga",
	is_doc_only = false,
})

function YuManga:init()
	self.ui.menu:registerToMainMenu(self)
	self.search_keyword = ""
	self.plugin_directory = debug.getinfo(1, "S").source:match("^@(.*/)")
	self.cover_cache_directory = DataStorage:getDataDir() .. "/cache/yumanga/covers"
	self.progress_path = DataStorage:getDataDir() .. "/cache/yumanga/download.progress"
	self.output_directory = G_reader_settings:readSetting("yumanga_output_directory")
		or G_reader_settings:readSetting("weebcentral_output_directory")
		or DataStorage:getFullDataDir() .. "/yumanga"
end

function YuManga:addToMainMenu(menu_items)
	UI.addToMainMenu(self, menu_items)
end

return YuManga
