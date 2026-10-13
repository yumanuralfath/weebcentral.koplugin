local ConfirmBox = require("ui/widget/confirmbox")
local Device = require("device")
local DownloadMgr = require("ui/downloadmgr")
local ImageWidget = require("ui/widget/imagewidget")
local InfoMessage = require("ui/widget/infomessage")
local InputDialog = require("ui/widget/inputdialog")
local Menu = require("ui/widget/menu")
local MultiInputDialog = require("ui/widget/multiinputdialog")
local ProgressbarDialog = require("ui/widget/progressbardialog")
local Trapper = require("ui/trapper")
local UIManager = require("ui/uimanager")
local NetworkMgr = require("ui/network/manager")
local Screen = Device.screen
local lfs = require("libs/libkoreader-lfs")
local _ = require("gettext")

local Core = require("lib/weebcentral/core")
local UI = {}

local function runWithProgress(plugin, message, command, progress_total, ...)
    local arguments = { ... }
    local progress
    if progress_total then
        progress = ProgressbarDialog:new({
            title = message,
            subtitle = _("Preparing download..."),
            progress_max = progress_total,
            refresh_time_seconds = 1,
        })
        progress:show()
    else
        progress = InfoMessage:new({
            text = message .. " (tap to cancel)",
        })
        UIManager:show(progress)
    end
    UIManager:forceRePaint()

    local stopped = false
    local poll_progress
    if progress_total then
        os.remove(plugin.progress_path)
        poll_progress = function()
            if stopped then return end
            local file = io.open(plugin.progress_path, "r")
            if file then
                local status = file:read("*l") or ""
                file:close()
                local current = status:match("^(%d+)|")
                local current_number = current and tonumber(current)
                if current_number then
                    progress:reportProgress(math.min(current_number, progress_total))
                end
            end
            UIManager:scheduleIn(0.5, poll_progress)
        end
        UIManager:scheduleIn(0.5, poll_progress)
    end

    local completed, output, succeeded = Trapper:dismissableRunInSubprocess(function()
        return Core.run(plugin.plugin_directory, command, unpack(arguments))
    end, progress)
    stopped = true
    if poll_progress then UIManager:unschedule(poll_progress) end
    os.remove(plugin.progress_path)
    if completed then
        if progress_total then progress:close() else UIManager:close(progress) end
    elseif progress_total then
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
            {
                text = _("Turn off Wi-Fi after download"),
                enabled_func = function()
                    return Device:hasWifiToggle()
                end,
                checked_func = function()
                    return G_reader_settings:isTrue("weebcentral_turn_off_wifi_after_download")
                end,
                callback = function()
                    local enabled = not G_reader_settings:isTrue("weebcentral_turn_off_wifi_after_download")
                    G_reader_settings:saveSetting("weebcentral_turn_off_wifi_after_download", enabled)
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
    local output, succeeded = runWithProgress(
        plugin,
        _("Searching..."),
        "search",
        nil,
        keyword,
        plugin.cover_cache_directory
    )
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
        local state
        if result.cover_path and result.cover_path ~= "" and lfs.attributes(result.cover_path, "mode") == "file" then
            state = ImageWidget:new({
                file = result.cover_path,
                width = Screen:scaleBySize(48),
                height = Screen:scaleBySize(64),
                scale_factor = 0,
            })
        end
        items[index] = {
            text = result.title,
            state = state,
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
                UI.loadMangaInfo(plugin, item.series)
            end)
        end,
    })
    UIManager:show(menu)
end

function UI.loadMangaInfo(plugin, series)
    local output, succeeded = runWithProgress(plugin, _("Loading chapters..."), "chapters", nil, series.id)
    local data, err = decodeCoreResponse(output, succeeded)
    if not data then
        if err then showMessage(err) end
        return
    end
    data.cover_path = series.cover_path
    UI.showMangaInfo(plugin, data)
end

function UI.showMangaInfo(plugin, series)
    local chapters = series.chapters or {}
    if #chapters == 0 then
        showMessage(_("No chapters found for ") .. series.title)
        return
    end

    local info = {
        series.title,
        "",
        _("Chapters: ") .. #chapters,
        _("Author: ") .. (series.authors ~= "" and series.authors or _("Unknown")),
        _("Type: ") .. (series.manga_type ~= "" and series.manga_type or _("Unknown")),
        _("Status: ") .. (series.status ~= "" and series.status or _("Unknown")),
    }
    if series.tags ~= "" then table.insert(info, _("Tags: ") .. series.tags) end
    if series.description ~= "" then
        table.insert(info, "")
        table.insert(info, series.description:sub(1, 700))
    end
    UIManager:show(ConfirmBox:new({
        text = table.concat(info, "\n"),
        ok_text = _("Choose chapters"),
        ok_callback = function()
            UI.openChapterSelection(plugin, series)
        end,
    }))
end

function UI.openChapterSelection(plugin, series)
    local dialog
    dialog = MultiInputDialog:new({
        title = _("Select chapters to download"),
        fields = {
            {
                text = plugin.last_selection or "",
                hint = _("1,3,4 or 1-10"),
                description = _("Chapter numbers: ") .. #series.chapters,
            },
            {
                text = plugin.last_volume_name or "",
                hint = _("Optional volume name, e.g. Vol 1"),
                description = _("Leave blank to create one CBZ per chapter."),
            },
        },
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
                    text = _("Download"),
                    is_enter_default = true,
                    callback = function()
                        local fields = dialog:getFields()
                        local selection = (fields[1] or ""):match("^%s*(.-)%s*$")
                        local volume_name = (fields[2] or ""):match("^%s*(.-)%s*$")
                        if selection == "" then
                            showMessage(_("Enter chapter numbers to download."))
                            return
                        end
                        plugin.last_selection = selection
                        plugin.last_volume_name = volume_name
                        UIManager:close(dialog)
                        Trapper:wrap(function()
                            UI.downloadSelection(plugin, series, selection, volume_name)
                        end)
                    end,
                },
            },
        },
    })
    UIManager:show(dialog)
    dialog:onShowKeyboard()
end

function UI.downloadSelection(plugin, series, selection, volume_name)
    local selection_count = UI.countSelectedChapters(selection, #series.chapters)
    if not selection_count then
        showMessage(_("Check the chapter selection and try again."))
        return
    end
    local output, succeeded = runWithProgress(
        plugin,
        _("Downloading selected chapters..."),
        "download",
        selection_count,
        series.id,
        selection,
        volume_name,
        plugin.output_directory,
        plugin.progress_path
    )
    local result, err = decodeCoreResponse(output, succeeded)
    if not result then
        if err then showMessage(err) end
        return
    end

    local message = _("Download complete") .. "\n\n" .. table.concat(result.paths or {}, "\n")
    if (result.skipped or 0) > 0 then
        message = message .. "\n" .. _("Existing files skipped: ") .. result.skipped
    end
    showMessage(message)
    if Device:hasWifiToggle() and G_reader_settings:isTrue("weebcentral_turn_off_wifi_after_download") then
        UIManager:nextTick(function()
            NetworkMgr:disableWifi()
        end)
    end
end

function UI.countSelectedChapters(selection, chapter_count)
    local selected = {}
    for part in selection:gmatch("[^,]+") do
        local first, last = part:match("^%s*(%d+)%s*%-%s*(%d+)%s*$")
        if first then
            first, last = tonumber(first), tonumber(last)
            if first < 1 or last < first or last > chapter_count then return nil end
            for index = first, last do selected[index] = true end
        else
            local chapter = tonumber(part:match("^%s*(%d+)%s*$"))
            if not chapter or chapter < 1 or chapter > chapter_count then return nil end
            selected[chapter] = true
        end
    end
    local count = 0
    for _ in pairs(selected) do count = count + 1 end
    return count > 0 and count or nil
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
