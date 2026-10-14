-- Search results: cover on the left, title and details on the right.
local Blitbuffer = require("ffi/blitbuffer")
local Device = require("device")
local Font = require("ui/font")
local Geom = require("ui/geometry")
local HorizontalGroup = require("ui/widget/horizontalgroup")
local HorizontalSpan = require("ui/widget/horizontalspan")
local LeftContainer = require("ui/widget/container/leftcontainer")
local CenterContainer = require("ui/widget/container/centercontainer")
local TextBoxWidget = require("ui/widget/textboxwidget")
local TextWidget = require("ui/widget/textwidget")
local VerticalGroup = require("ui/widget/verticalgroup")
local VerticalSpan = require("ui/widget/verticalspan")
local T = require("ffi/util").template
local _ = require("gettext")
local Screen = Device.screen

local Format = require("lib/weebcentral/format")
local PagedList = require("lib/weebcentral/paged_list")
local Widgets = require("lib/weebcentral/widgets")

local ResultsView = PagedList:extend({
	results = nil,
	on_select = nil, -- function(result)
})

function ResultsView:init()
	self.items = self.results or {}
	self.title = T(_("Search results (%1)"), #self.items)

	self.pad = Screen:scaleBySize(8)
	self.title_face = Font:getFace("cfont", 19)
	self.sub_face = Font:getFace("smallinfofont", 15)
	self.title_lh = Widgets.lineHeight(self.title_face)
	self.sub_lh = Widgets.lineHeight(self.sub_face)

	-- Enough room for a two line title plus three detail lines.
	local line_gap = Screen:scaleBySize(3)
	local text_need = 2 * self.title_lh + 3 * self.sub_lh + 3 * line_gap
	local sep = Screen:scaleBySize(2)
	self.sep_height = sep
	self.line_gap = line_gap
	self.row_height = math.max(math.floor(Screen:getHeight() * 0.12), text_need + 2 * self.pad + sep)
	PagedList.init(self)
end

function ResultsView:buildRow(result, index)
	local pad = self.pad
	local cover_h = self.row_height - 2 * pad - self.sep_height
	local cover = Widgets.cover(result.cover_path, math.floor(cover_h * 0.75), cover_h)
	local cover_w = cover:getSize().w
	local chevron_w = Screen:scaleBySize(22)
	local text_w = self.width - cover_w - chevron_w - 4 * pad

	local authors = Format.clean(result.authors)
	local status = Format.clean(result.status)
	local manga_type = Format.clean(result.manga_type)
	local released = Format.clean(result.released)
	local tag_summary = Format.tagSummary(result.tags, 4)

	local lines = VerticalGroup:new({ align = "left" })
	table.insert(
		lines,
		TextBoxWidget:new({
			text = result.title,
			width = text_w,
			face = self.title_face,
			bold = true,
			alignment = "left",
			height = 2 * self.title_lh,
			height_adjust = true,
			height_overflow_show_ellipsis = true,
		})
	)
	if authors ~= "" then
		table.insert(lines, VerticalSpan:new({ width = self.line_gap }))
		table.insert(
			lines,
			TextWidget:new({
				text = authors,
				face = self.sub_face,
				fgcolor = Blitbuffer.COLOR_DARK_GRAY,
				max_width = text_w,
			})
		)
	end
	if status ~= "" or manga_type ~= "" or released ~= "" then
		table.insert(lines, VerticalSpan:new({ width = self.line_gap }))
		local meta = HorizontalGroup:new({ align = "center" })
		if status ~= "" then
			table.insert(meta, Widgets.statusBadge(status, { face = Font:getFace("xx_smallinfofont") }))
			table.insert(meta, HorizontalSpan:new({ width = pad }))
		end
		local meta_text = {}
		if manga_type ~= "" then
			meta_text[#meta_text + 1] = manga_type
		end
		if released ~= "" then
			meta_text[#meta_text + 1] = released
		end
		if #meta_text > 0 then
			table.insert(
				meta,
				TextWidget:new({
					text = table.concat(meta_text, " · "),
					face = self.sub_face,
					fgcolor = Blitbuffer.COLOR_DARK_GRAY,
					max_width = text_w - (status ~= "" and Screen:scaleBySize(110) or 0),
				})
			)
		end
		table.insert(lines, meta)
	end
	if tag_summary ~= "" then
		table.insert(lines, VerticalSpan:new({ width = self.line_gap }))
		table.insert(
			lines,
			TextWidget:new({
				text = tag_summary,
				face = self.sub_face,
				fgcolor = Blitbuffer.COLOR_GRAY_5 or Blitbuffer.COLOR_DARK_GRAY,
				max_width = text_w,
			})
		)
	end

	local content = HorizontalGroup:new({
		align = "center",
		HorizontalSpan:new({ width = pad }),
		cover,
		HorizontalSpan:new({ width = pad * 2 }),
		LeftContainer:new({ dimen = Geom:new({ w = text_w, h = cover_h }), lines }),
		CenterContainer:new({
			dimen = Geom:new({ w = chevron_w, h = cover_h }),
			TextWidget:new({ text = "›", face = Font:getFace("cfont", 28) }),
		}),
		HorizontalSpan:new({ width = pad }),
	})

	local row_body = VerticalGroup:new({
		align = "left",
		VerticalSpan:new({ width = pad }),
		content,
		VerticalSpan:new({ width = pad }),
		Widgets.separator(self.width),
	})

	return PagedList.Row:new({
		width = self.width,
		height = self.row_height,
		content = row_body,
		on_tap = function()
			if self.on_select then
				self.on_select(result, index)
			end
		end,
	})
end

return ResultsView
