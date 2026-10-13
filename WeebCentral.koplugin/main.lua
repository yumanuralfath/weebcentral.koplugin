local InfoMessage = require("ui/widget/infomessage")
local ConfirmBox = require("ui/widget/confirmbox")
local InputDialog = require("ui/widget/inputdialog")
local Menu = require("ui/widget/menu")
local Trapper = require("ui/trapper")
local UIManager = require("ui/uimanager")
local DataStorage = require("datastorage")
local _ = require("gettext")
local rapidjson = require("rapidjson")
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
	self.search_keyword = ""
	self.output_directory = G_reader_settings:readSetting("weebcentral_output_directory")
		or DataStorage:getFullDataDir() .. "/weebcentral"
end

function Weeb:addToMainMenu(menu_items)
	menu_items.weeb_central = {
		text = _("WeebCentral Downloader"),
		sorting_hint = "tools",
		sub_item_table = {
			{
				text = _("Cari manga"),
				callback = function()
					self:openSearchDialog()
				end,
			},
			{
				text = _("Folder download"),
				callback = function()
					self:openOutputDirectoryDialog()
				end,
			},
		},
	}
end

function Weeb:getBinaryPath()
	local source = debug.getinfo(1, "S").source
	local plugin_directory = source:match("^@(.*/)")
	if not plugin_directory then
		return nil
	end
	return plugin_directory .. "bin/weebcentral_core_dev"
end

function Weeb:runBinary(...)
	local binary = self:getBinaryPath()
	if not binary then
		return nil, false
	end
	local invocation = shellQuote(binary)
	for _, argument in ipairs({ ... }) do
		invocation = invocation .. " " .. shellQuote(tostring(argument))
	end
	local pipe = io.popen(invocation .. " 2>&1", "r")
	if not pipe then
		return nil, false
	end

	local output = pipe:read("*a") or ""
	local succeeded = pipe:close()
	return output, succeeded == true
end

function Weeb:runWithProgress(message, callback)
	local progress = InfoMessage:new({
		text = message .. " (ketuk untuk membatalkan)",
	})
	UIManager:show(progress)
	UIManager:forceRePaint()

	local completed, output, succeeded = Trapper:dismissableRunInSubprocess(callback, progress)
	if completed then
		UIManager:close(progress)
	end
	if not completed then
		return nil, false
	end
	return output, succeeded
end

function Weeb:openSearchDialog()
	local dialog
	dialog = InputDialog:new({
		title = _("Cari manga di WeebCentral"),
		input = self.search_keyword,
		input_hint = _("Judul manga"),
		buttons = {
			{
				{
					text = _("Batal"),
					id = "close",
					callback = function()
						UIManager:close(dialog)
					end,
				},
				{
					text = _("Cari"),
					is_enter_default = true,
					callback = function()
						self.search_keyword = dialog:getInputText():match("^%s*(.-)%s*$")
						UIManager:close(dialog)
						if self.search_keyword ~= "" then
							Trapper:wrap(function()
								self:search(self.search_keyword)
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

function Weeb:search(keyword)
	local output, succeeded = self:runWithProgress(_("Sedang mencari..."), function()
		return self:runBinary("search", keyword)
	end)
	local results, err = self:decodeCoreResponse(output, succeeded)
	if not results then
		if err then self:showMessage(err) end
		return
	end
	self:showSearchResults(results)
end

function Weeb:loadChapters(series)
	local output, succeeded = self:runWithProgress(_("Memuat daftar chapter..."), function()
		return self:runBinary("chapters", series.id)
	end)
	local data, err = self:decodeCoreResponse(output, succeeded)
	if not data then
		if err then self:showMessage(err) end
		return
	end
	self:showChapters(data)
end

function Weeb:decodeCoreResponse(output, succeeded)
	if not output then
		return nil, _("Tidak dapat menjalankan binary WeebCentral.")
	end
	local response, decode_error = rapidjson.decode(output)
	if not response then
		return nil, _("Respons core tidak valid: ") .. tostring(decode_error) .. "\n\n" .. output
	end
	if not succeeded or response.ok ~= true then
		return nil, response.error or _("Operasi WeebCentral gagal.")
	end
	return response.data
end

function Weeb:showSearchResults(results)
	if #results == 0 then
		self:showMessage(_("Tidak ada hasil ditemukan."))
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

	local menu
	menu = Menu:new({
		title = _("Hasil pencarian"),
		item_table = items,
		covers_fullscreen = true,
		onMenuSelect = function(menu_self, item)
			UIManager:close(menu_self)
			Trapper:wrap(function()
				self:loadChapters(item.series)
			end)
		end,
	})
	UIManager:show(menu)
end

function Weeb:showChapters(series)
	local chapters = series.chapters or {}
	if #chapters == 0 then
		self:showMessage(_("Tidak ada chapter ditemukan untuk ") .. series.title)
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

	local menu
	menu = Menu:new({
		title = series.title,
		item_table = items,
		covers_fullscreen = true,
		onMenuSelect = function(menu_self, item)
			UIManager:close(menu_self)
			self:confirmDownload(item.chapter)
		end,
	})
	UIManager:show(menu)
end

function Weeb:confirmDownload(chapter)
	UIManager:show(ConfirmBox:new({
		text = _("Download sebagai CBZ?") .. "\n\n" .. chapter.title .. "\n" .. self.output_directory,
		ok_text = _("Download"),
		ok_callback = function()
			Trapper:wrap(function()
				self:downloadChapter(chapter)
			end)
		end,
	}))
end

function Weeb:downloadChapter(chapter)
	local output, succeeded = self:runWithProgress(_("Mengunduh chapter..."), function()
		return self:runBinary("download", chapter.id, chapter.title, self.output_directory)
	end)
	local result, err = self:decodeCoreResponse(output, succeeded)
	if not result then
		if err then self:showMessage(err) end
		return
	end
	self:showMessage(_("CBZ tersimpan di:") .. "\n" .. result.path)
end

function Weeb:openOutputDirectoryDialog()
	local dialog
	dialog = InputDialog:new({
		title = _("Folder download CBZ"),
		input = self.output_directory,
		input_hint = _("Path folder di perangkat"),
		buttons = {
			{
				{
					text = _("Batal"),
					id = "close",
					callback = function()
						UIManager:close(dialog)
					end,
				},
				{
					text = _("Simpan"),
					is_enter_default = true,
					callback = function()
						local path = dialog:getInputText():match("^%s*(.-)%s*$")
						if path ~= "" then
							self.output_directory = path
							G_reader_settings:saveSetting("weebcentral_output_directory", path)
						end
						UIManager:close(dialog)
					end,
				},
			},
		},
	})
	UIManager:show(dialog)
	dialog:onShowKeyboard()
end

function Weeb:showMessage(message)
	local popup = InfoMessage:new({
		text = message,
	})
	UIManager:show(popup)
end

return Weeb
