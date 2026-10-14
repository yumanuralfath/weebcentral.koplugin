-- Chapter list with checkboxes. Tap toggles a chapter, long-press selects
-- every chapter between the last tapped one and this one.
local Blitbuffer = require("ffi/blitbuffer")
local ButtonTable = require("ui/widget/buttontable")
local CenterContainer = require("ui/widget/container/centercontainer")
local Device = require("device")
local Font = require("ui/font")
local FrameContainer = require("ui/widget/container/framecontainer")
local Geom = require("ui/geometry")
local HorizontalGroup = require("ui/widget/horizontalgroup")
local HorizontalSpan = require("ui/widget/horizontalspan")
local InputDialog = require("ui/widget/inputdialog")
local InfoMessage = require("ui/widget/infomessage")
local LeftContainer = require("ui/widget/container/leftcontainer")
local RightContainer = require("ui/widget/container/rightcontainer")
local Size = require("ui/size")
local TextWidget = require("ui/widget/textwidget")
local UIManager = require("ui/uimanager")
local VerticalGroup = require("ui/widget/verticalgroup")
local VerticalSpan = require("ui/widget/verticalspan")
local T = require("ffi/util").template
local _ = require("gettext")
local Screen = Device.screen

local Format = require("lib/weebcentral/format")
local PagedList = require("lib/weebcentral/paged_list")
local Widgets = require("lib/weebcentral/widgets")

local ChapterPicker = PagedList:extend({
	series = nil,
	on_download = nil, -- function(picker, selection_string, count)
})

function ChapterPicker:init()
	self.chapters = self.series.chapters or {}
	self.selected = {}
	self.selected_count = 0
	self.anchor = nil
	self.newest_first = true
	self.title = self.series.title
	self.subtitle = _("Tap to select · hold to select a range")

	self.pad = Screen:scaleBySize(10)
	self.name_face = Font:getFace("cfont", 18)
	self.small_face = Font:getFace("x_smallinfofont")
	self.row_height = math.max(Widgets.lineHeight(self.name_face) + 2 * self.pad, Screen:scaleBySize(52))
	self.box = math.floor(self.row_height * 0.42)

	self:buildItems()
	PagedList.init(self)
end

-- Items are chapter positions (1 = oldest), ordered for display.
function ChapterPicker:buildItems()
	local items = {}
	local total = #self.chapters
	for position = 1, total do
		items[position] = self.newest_first and (total - position + 1) or position
	end
	self.items = items
end

function ChapterPicker:setSelected(index, value)
	if value and not self.selected[index] then
		self.selected[index] = true
		self.selected_count = self.selected_count + 1
	elseif not value and self.selected[index] then
		self.selected[index] = nil
		self.selected_count = self.selected_count - 1
	end
end

function ChapterPicker:clearSelection()
	self.selected = {}
	self.selected_count = 0
	self.anchor = nil
	self:refresh()
end

function ChapterPicker:selectRange(first, last)
	if first > last then
		first, last = last, first
	end
	for index = first, last do
		self:setSelected(index, true)
	end
end

function ChapterPicker:checkbox(checked)
	local inner = math.floor(self.box * 0.6)
	local padding = math.floor(self.box * 0.2)
	return FrameContainer:new({
		bordersize = Size.border.default,
		margin = 0,
		padding = padding,
		radius = Size.radius.default,
		background = Blitbuffer.COLOR_WHITE,
		FrameContainer:new({
			bordersize = 0,
			margin = 0,
			padding = 0,
			width = inner,
			height = inner,
			background = checked and Blitbuffer.COLOR_BLACK or Blitbuffer.COLOR_WHITE,
			VerticalSpan:new({ width = inner }),
		}),
	})
end

-- `position` is the chapter's place in the list (1 = oldest).
function ChapterPicker:buildRow(position)
	local index = position
	local chapter = self.chapters[index]
	local pad = self.pad
	local checked = self.selected[index] == true
	local number_w = Screen:scaleBySize(56)
	local date = Format.isoDate(chapter.published_at)
	local date_w = date ~= "" and Screen:scaleBySize(110) or 0
	local box_w = self.box + 2 * Size.border.default
	local name_w = self.width - box_w - number_w - date_w - 5 * pad

	local content = HorizontalGroup:new({
		align = "center",
		HorizontalSpan:new({ width = pad }),
		self:checkbox(checked),
		HorizontalSpan:new({ width = pad }),
		LeftContainer:new({
			dimen = Geom:new({ w = number_w, h = self.row_height - 2 }),
			TextWidget:new({
				text = "#" .. index,
				face = self.small_face,
				fgcolor = Blitbuffer.COLOR_DARK_GRAY,
				max_width = number_w,
			}),
		}),
		LeftContainer:new({
			dimen = Geom:new({ w = name_w, h = self.row_height - 2 }),
			TextWidget:new({
				text = chapter.title,
				face = self.name_face,
				bold = checked,
				max_width = name_w,
			}),
		}),
		RightContainer:new({
			dimen = Geom:new({ w = date_w + pad, h = self.row_height - 2 }),
			TextWidget:new({
				text = date,
				face = self.small_face,
				fgcolor = Blitbuffer.COLOR_DARK_GRAY,
				max_width = date_w,
			}),
		}),
	})

	return PagedList.Row:new({
		width = self.width,
		height = self.row_height,
		content = VerticalGroup:new({
			align = "left",
			CenterContainer:new({
				dimen = Geom:new({ w = self.width, h = self.row_height - 2 }),
				content,
			}),
			Widgets.separator(self.width),
		}),
		on_tap = function()
			self:setSelected(index, not self.selected[index])
			self.anchor = index
			self:refresh()
		end,
		on_hold = function()
			if self.anchor then
				self:selectRange(self.anchor, index)
			else
				self:setSelected(index, true)
			end
			self.anchor = index
			self:refresh()
		end,
	})
end

function ChapterPicker:buildActionBar()
	local total = #self.chapters
	local download_text = self.selected_count > 0 and T(_("Download (%1)"), self.selected_count) or _("Download")
	return ButtonTable:new({
		width = self.width,
		show_parent = self,
		buttons = {
			{
				{
					text = _("Select all"),
					callback = function()
						self:selectRange(1, total)
						self:refresh()
					end,
				},
				{
					text = _("Clear"),
					enabled = self.selected_count > 0,
					callback = function()
						self:clearSelection()
					end,
				},
				{
					text = _("Range…"),
					callback = function()
						self:askRange()
					end,
				},
			},
			{
				{
					text = self.newest_first and _("Newest first ▾") or _("Oldest first ▴"),
					callback = function()
						self.newest_first = not self.newest_first
						self:buildItems()
						self.page = 1
						self:refresh("partial")
					end,
				},
				{
					text = download_text,
					enabled = self.selected_count > 0,
					callback = function()
						if self.on_download then
							self.on_download(self, Format.compressSelection(self.selected), self.selected_count)
						end
					end,
				},
			},
		},
	})
end

function ChapterPicker:askRange()
	local total = #self.chapters
	local dialog
	dialog = InputDialog:new({
		title = _("Select chapters"),
		input_hint = _("1,3,4 or 1-10"),
		description = T(_("Chapter numbers (#) from 1 to %1. They are added to the current selection."), total),
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
					text = _("Add"),
					is_enter_default = true,
					callback = function()
						local picked = Format.parseSelection(dialog:getInputText(), total)
						if not picked then
							UIManager:show(InfoMessage:new({ text = _("Check the chapter selection and try again.") }))
							return
						end
						UIManager:close(dialog)
						for index in pairs(picked) do
							self:setSelected(index, true)
						end
						self:refresh()
					end,
				},
			},
		},
	})
	UIManager:show(dialog)
	dialog:onShowKeyboard()
end

return ChapterPicker
