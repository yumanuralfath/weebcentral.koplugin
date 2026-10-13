local ConfirmBox = require("ui/widget/confirmbox")
local DownloadMgr = require("ui/downloadmgr")
local InfoMessage = require("ui/widget/infomessage")
local InputDialog = require("ui/widget/inputdialog")
local Menu = require("ui/widget/menu")
local Trapper = require("ui/trapper")
local UIManager = require("ui/uimanager")
local _ = require("gettext")

local Core = require("lib/weebcentral/core")
local UI = {}

local function runWithProgress(plugin, message, command, ...)
    local arguments = { ... }
    local progress = InfoMessage:new({
        text = message .. " (tap to cancel)",
    })
    UIManager:show(progress)
    UIManager:forceRePaint()

    local completed, output, succeeded = Trapper:dismissableRunInSubprocess(function()
        return Core.run(plugin.plugin_directory, command, unpack(arguments))
    end, progress)
    if completed then
        UIManager:close(progress)
    end
    if not completed then
        return nil, false
    end
    return output, succeeded
end

local function showMessage(message)
    UIManager:show(InfoMessage:new({ text = message }))
end

local function decodeCoreResponse(output, succeeded)
    return Core.decodeResponse(output, succeeded)
end

function UI.addToMainMenu(plugin, menu_items)
    menu_items.weeb_central = {
        text = _("WeebCentral Downloader"),
        sorting_hint = "tools",
        sub_item_table = {
            {
                text = _("Search manga"),
                callback = function()
                    UI.openSearchDialog(plugin)
                end,
            },
            {
                text = _("Choose download folder"),
                callback = function()
                    UI.chooseOutputDirectory(plugin)
                end,
            },
        },
    }
end

function UI.openSearchDialog(plugin)
    local dialog
    dialog = InputDialog:new({
        title = _("Search WeebCentral"),
        input = plugin.search_keyword,
        input_hint = _("Manga title"),
        buttons = {
            {
                {
                    text = _("Cancel"),
                    id = "close",
                    callback = function()
                        UIManager:close(dialog)
                    end,
                },
                {
                    text = _("Search"),
                    is_enter_default = true,
                    callback = function()
                        plugin.search_keyword = dialog:getInputText():match("^%s*(.-)%s*$")
                        UIManager:close(dialog)
                        if plugin.search_keyword ~= "" then
                            Trapper:wrap(function()
                                UI.search(plugin, plugin.search_keyword)
                            end)
                        end
                    end,
                },
            },
        },
    })
    UIManager:show(dialog)
    dialog:onShowKeyboard()
end

function UI.search(plugin, keyword)
    local output, succeeded = runWithProgress(plugin, _("Searching..."), "search", keyword)
    local results, err = decodeCoreResponse(output, succeeded)
    if not results then
        if err then showMessage(err) end
        return
    end
    UI.showSearchResults(plugin, results)
end

function UI.showSearchResults(plugin, results)
    if #results == 0 then
        showMessage(_("No manga found."))
        return
    end

    local items = {}
    for index, result in ipairs(results) do
        items[index] = {
            text = result.title,
            mandatory = result.id,
            series = result,
        }
    end

    local menu = Menu:new({
        title = _("Search results"),
        item_table = items,
        covers_fullscreen = true,
        onMenuSelect = function(menu_self, item)
            UIManager:close(menu_self)
            Trapper:wrap(function()
                UI.loadChapters(plugin, item.series)
            end)
        end,
    })
    UIManager:show(menu)
end

function UI.loadChapters(plugin, series)
    local output, succeeded = runWithProgress(plugin, _("Loading chapters..."), "chapters", series.id)
    local data, err = decodeCoreResponse(output, succeeded)
    if not data then
        if err then showMessage(err) end
        return
    end
    UI.showChapters(plugin, data)
end

function UI.showChapters(plugin, series)
    local chapters = series.chapters or {}
    if #chapters == 0 then
        showMessage(_("No chapters found for ") .. series.title)
        return
    end

    local items = {}
    for index, chapter in ipairs(chapters) do
        items[index] = {
            text = chapter.title,
            mandatory = chapter.published_at ~= "" and chapter.published_at or chapter.id,
            chapter = chapter,
        }
    end

    local menu = Menu:new({
        title = series.title,
        item_table = items,
        covers_fullscreen = true,
        onMenuSelect = function(menu_self, item)
            UIManager:close(menu_self)
            UI.confirmDownload(plugin, item.chapter)
        end,
    })
    UIManager:show(menu)
end

function UI.confirmDownload(plugin, chapter)
    UIManager:show(ConfirmBox:new({
        text = _("Download as CBZ?") .. "\n\n" .. chapter.title .. "\n" .. plugin.output_directory,
        ok_text = _("Download"),
        ok_callback = function()
            Trapper:wrap(function()
                UI.downloadChapter(plugin, chapter)
            end)
        end,
    }))
end

function UI.downloadChapter(plugin, chapter)
    local output, succeeded = runWithProgress(
        plugin,
        _("Downloading chapter..."),
        "download",
        chapter.id,
        chapter.title,
        plugin.output_directory
    )
    local result, err = decodeCoreResponse(output, succeeded)
    if not result then
        if err then showMessage(err) end
        return
    end

    local message = result.already_exists and _("CBZ already exists:") or _("CBZ saved to:")
    showMessage(message .. "\n" .. result.path)
end

function UI.chooseOutputDirectory(plugin)
    DownloadMgr:new({
        title = _("Choose download folder"),
        onConfirm = function(path)
            plugin.output_directory = path
            G_reader_settings:saveSetting("weebcentral_output_directory", path)
        end,
    }):chooseDir(plugin.output_directory)
end

return UI
