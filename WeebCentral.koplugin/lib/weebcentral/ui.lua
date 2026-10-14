local Device = require("device")
local ButtonDialog = require("ui/widget/buttondialog")
local DownloadMgr = require("ui/downloadmgr")
local HorizontalGroup = require("ui/widget/horizontalgroup")
local HorizontalSpan = require("ui/widget/horizontalspan")
local ImageWidget = require("ui/widget/imagewidget")
local InfoMessage = require("ui/widget/infomessage")
local InputDialog = require("ui/widget/inputdialog")
local Menu = require("ui/widget/menu")
local MultiInputDialog = require("ui/widget/multiinputdialog")
local ProgressbarDialog = require("ui/widget/progressbardialog")
local TextBoxWidget = require("ui/widget/textboxwidget")
local TextWidget = require("ui/widget/textwidget")
local Trapper = require("ui/trapper")
local UIManager = require("ui/uimanager")
local NetworkMgr = require("ui/network/manager")
local VerticalGroup = require("ui/widget/verticalgroup")
local VerticalSpan = require("ui/widget/verticalspan")
local Screen = Device.screen
local lfs = require("libs/libkoreader-lfs")
local ffiUtil = require("ffi/util")
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
			if stopped then
				return
			end
			local file = io.open(plugin.progress_path, "r")
			if file then
				local status = file:read("*l") or ""
				file:close()
				local stage, current, total = status:match("^(%a+)|(%d+)|(%d+)|")
				current, total = tonumber(current), tonumber(total)
				if current and total and total > 0 then
					local stage_fraction = math.min(current / total, 1)
					local fraction = stage == "scan" and stage_fraction * 0.1 or 0.1 + stage_fraction * 0.9
					progress:reportProgress(fraction * progress_total)
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
	if poll_progress then
		UIManager:unschedule(poll_progress)
	end
	os.remove(plugin.progress_path)
	if completed then
		if progress_total then
			progress:close()
		else
			UIManager:close(progress)
		end
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
				text = _("Clear Cache"),
				callback = function()
					UI.clearCoverCache(plugin)
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
	local output, succeeded =
		runWithProgress(plugin, _("Searching..."), "search", nil, keyword, plugin.cover_cache_directory)
	local results, err = decodeCoreResponse(output, succeeded)
	if not results then
		if err then
			showMessage(err)
		end
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
	local output, succeeded = runWithProgress(plugin, _("Loading info..."), "chapters", nil, series.id)
	local data, err = decodeCoreResponse(output, succeeded)
	if not data then
		if err then
			showMessage(err)
		end
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

	local dialog_width = math.floor(math.min(Screen:getWidth(), Screen:getHeight()) * 0.9)
	local cover_width = math.min(Screen:scaleBySize(112), math.floor(dialog_width * 0.27))
	local cover_height = math.min(Screen:scaleBySize(158), math.floor(Screen:getHeight() * 0.27))
	local horizontal_gap = Screen:scaleBySize(12)
	local text_width = dialog_width - cover_width - horizontal_gap - Screen:scaleBySize(36)
	local header_parts = VerticalGroup:new({
		align = "left",
		TextBoxWidget:new({
			text = series.title,
			width = text_width,
			face = require("ui/font"):getFace("cfont", 22),
			bold = true,
			alignment = "left",
			height = Screen:scaleBySize(66),
			height_adjust = true,
			height_overflow_show_ellipsis = true,
		}),
		VerticalSpan:new({ width = Screen:scaleBySize(8) }),
		TextWidget:new({
			text = tostring(#chapters) .. " " .. _("chapters"),
			face = require("ui/font"):getFace("smallinfofont"),
		}),
		TextBoxWidget:new({
			text = _("Author: ") .. (series.authors ~= "" and series.authors or _("Unknown")),
			width = text_width,
			face = require("ui/font"):getFace("smallinfofont"),
			height = Screen:scaleBySize(42),
			height_adjust = true,
			height_overflow_show_ellipsis = true,
		}),
		TextBoxWidget:new({
			text = (series.manga_type ~= "" and series.manga_type or _("Unknown"))
				.. " · "
				.. (series.status ~= "" and series.status or _("Unknown")),
			width = text_width,
			face = require("ui/font"):getFace("smallinfofont"),
			height = Screen:scaleBySize(42),
			height_adjust = true,
			height_overflow_show_ellipsis = true,
		}),
	})

	local cover_widget
	if series.cover_path and series.cover_path ~= "" and lfs.attributes(series.cover_path, "mode") == "file" then
		cover_widget = ImageWidget:new({
			file = series.cover_path,
			width = cover_width,
			height = cover_height,
			scale_factor = 0,
			file_do_cache = true,
			alpha = false,
		})
	else
		cover_widget = TextWidget:new({
			text = _("No cover"),
			face = require("ui/font"):getFace("smallinfofont"),
			max_width = cover_width,
		})
	end

	local header = HorizontalGroup:new({
		align = "top",
		cover_widget,
		HorizontalSpan:new({ width = horizontal_gap }),
		header_parts,
	})
	local content = VerticalGroup:new({
		align = "left",
		header,
		VerticalSpan:new({ width = Screen:scaleBySize(12) }),
	})
	if series.tags ~= "" then
		table.insert(
			content,
			TextBoxWidget:new({
				text = _("Tags: ") .. series.tags,
				width = dialog_width - Screen:scaleBySize(36),
				face = require("ui/font"):getFace("smallinfofont"),
				height = Screen:scaleBySize(54),
				height_adjust = true,
				height_overflow_show_ellipsis = true,
			})
		)
	end
	if series.description ~= "" then
		table.insert(content, VerticalSpan:new({ width = Screen:scaleBySize(8) }))
		table.insert(
			content,
			TextBoxWidget:new({
				text = series.description:sub(1, 650),
				width = dialog_width - Screen:scaleBySize(36),
				face = require("ui/font"):getFace("smallinfofont"),
				height = math.floor(Screen:getHeight() * 0.22),
				height_adjust = true,
				height_overflow_show_ellipsis = true,
				alignment = "left",
			})
		)
	end
	local dialog
	dialog = ButtonDialog:new({
		title = _("Manga details"),
		buttons = {
			{
				{
					text = _("Close"),
					id = "close",
					callback = function()
						UIManager:close(dialog)
					end,
				},
				{
					text = _("Choose chapters"),
					is_enter_default = true,
					callback = function()
						UIManager:close(dialog)
						UI.openChapterSelection(plugin, series)
					end,
				},
			},
		},
		_added_widgets = { content },
	})
	UIManager:show(dialog)
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
		1000,
		series.id,
		selection,
		volume_name,
		plugin.output_directory,
		plugin.progress_path
	)
	local result, err = decodeCoreResponse(output, succeeded)
	if not result then
		if err then
			showMessage(err)
		end
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
			if first < 1 or last < first or last > chapter_count then
				return nil
			end
			for index = first, last do
				selected[index] = true
			end
		else
			local chapter = tonumber(part:match("^%s*(%d+)%s*$"))
			if not chapter or chapter < 1 or chapter > chapter_count then
				return nil
			end
			selected[chapter] = true
		end
	end
	local count = 0
	for _ in pairs(selected) do
		count = count + 1
	end
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

function UI.clearCoverCache(plugin)
	ffiUtil.purgeDir(plugin.cover_cache_directory)
	showMessage("Cache Cleared")
end

return UI
