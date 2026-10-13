local DataStorage = require("datastorage")
local widgetcontainer = require("ui.widget.container.widgetcontainer")
local UI = require("lib/weebcentral/ui")

local WeebCentral = widgetcontainer:extend({
    name = "weeb_central",
    is_doc_only = false,
})

function WeebCentral:init()
    self.ui.menu:registerToMainMenu(self)
    self.search_keyword = ""
    self.plugin_directory = debug.getinfo(1, "S").source:match("^@(.*/)")
    self.cover_cache_directory = DataStorage:getDataDir() .. "/cache/weebcentral/covers"
    self.progress_path = DataStorage:getDataDir() .. "/cache/weebcentral/download.progress"
    self.output_directory = G_reader_settings:readSetting("weebcentral_output_directory")
        or DataStorage:getFullDataDir() .. "/weebcentral"
end

function WeebCentral:addToMainMenu(menu_items)
    UI.addToMainMenu(self, menu_items)
end

return WeebCentral
