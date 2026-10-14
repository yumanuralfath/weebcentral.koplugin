-- Manga detail page: cover + metadata header, tags, quick actions and synopsis.
local Blitbuffer = require("ffi/blitbuffer")
local ButtonTable = require("ui/widget/buttontable")
local Device = require("device")
local Font = require("ui/font")
local FrameContainer = require("ui/widget/container/framecontainer")
local Geom = require("ui/geometry")
local HorizontalGroup = require("ui/widget/horizontalgroup")
local HorizontalSpan = require("ui/widget/horizontalspan")
local InputContainer = require("ui/widget/container/inputcontainer")
local ScrollTextWidget = require("ui/widget/scrolltextwidget")
local TextBoxWidget = require("ui/widget/textboxwidget")
local TextWidget = require("ui/widget/textwidget")
local TitleBar = require("ui/widget/titlebar")
local UIManager = require("ui/uimanager")
local VerticalGroup = require("ui/widget/verticalgroup")
local VerticalSpan = require("ui/widget/verticalspan")
local T = require("ffi/util").template
local _ = require("gettext")
local Screen = Device.screen

local Format = require("lib/weebcentral/format")
local Widgets = require("lib/weebcentral/widgets")

local DetailView = InputContainer:extend({
	covers_fullscreen = true,
	series = nil,
	on_choose = nil, -- open the chapter picker
	on_enter_numbers = nil, -- type chapter numbers manually
	on_download = nil, -- function(selection, count): quick downloads
	on_close = nil,
	on_quick = nil, -- open the First/Latest dialog
})

function DetailView:init()
	self.width = Screen:getWidth()
	self.height = Screen:getHeight()
	self.dimen = Geom:new({ x = 0, y = 0, w = self.width, h = self.height })
	if Device:hasKeys() then
		self.key_events.Close = { { Device.input.group.Back } }
	end
	self:build()
end

local function infoLine(label, value, face, width)
	return TextWidget:new({
		text = label .. ": " .. value,
		face = face,
		fgcolor = Blitbuffer.COLOR_DARK_GRAY,
		max_width = width,
	})
end

function DetailView:build()
	local series = self.series
	local chapters = series.chapters or {}
	local total = #chapters
	local pad = Screen:scaleBySize(14)
	local inner_w = self.width - 2 * pad
	local gap = Screen:scaleBySize(8)

	local title_bar = TitleBar:new({
		width = self.width,
		fullscreen = true,
		with_bottom_line = true,
		title = _("Manga details"),
		close_callback = function()
			self:onClose()
		end,
		show_parent = self,
	})

	-- Header: cover on the left, facts on the right.
	local cover_h =
		math.min(math.floor(math.min(self.width, self.height) * 0.30 * 1.45), math.floor(self.height * 0.30))
	local cover = Widgets.cover(series.cover_path, math.floor(cover_h * 0.75), cover_h)
	local text_w = inner_w - cover:getSize().w - pad

	local title_face = Font:getFace("cfont", 22)
	local sub_face = Font:getFace("smallinfofont", 16)
	local title_lh = Widgets.lineHeight(title_face)

	local facts = VerticalGroup:new({ align = "left" })
	table.insert(
		facts,
		TextBoxWidget:new({
			text = series.title,
			width = text_w,
			face = title_face,
			bold = true,
			alignment = "left",
			height = 3 * title_lh,
			height_adjust = true,
			height_overflow_show_ellipsis = true,
		})
	)
	table.insert(facts, VerticalSpan:new({ width = gap }))
	local authors = Format.orElse(series.authors, _("Unknown"))
	table.insert(
		facts,
		TextBoxWidget:new({
			text = T(_("by %1"), authors),
			width = text_w,
			face = sub_face,
			alignment = "left",
			height = 2 * Widgets.lineHeight(sub_face),
			height_adjust = true,
			height_overflow_show_ellipsis = true,
		})
	)
	table.insert(facts, VerticalSpan:new({ width = gap }))

	local badges = HorizontalGroup:new({ align = "center" })
	local status = Format.clean(series.status)
	local manga_type = Format.clean(series.manga_type)
	if status ~= "" then
		table.insert(badges, Widgets.statusBadge(status))
		table.insert(badges, HorizontalSpan:new({ width = gap }))
	end
	if manga_type ~= "" then
		table.insert(badges, Widgets.badge(manga_type))
	end
	if #badges > 0 then
		table.insert(facts, badges)
		table.insert(facts, VerticalSpan:new({ width = gap }))
	end

	local small = Font:getFace("x_smallinfofont")
	local released = Format.clean(series.released)
	if released ~= "" then
		table.insert(facts, infoLine(_("Released"), released, small, text_w))
	end
	table.insert(facts, infoLine(_("Chapters"), tostring(total), small, text_w))
	local last_update = Format.lastUpdate(chapters)
	if last_update ~= "" then
		table.insert(facts, infoLine(_("Last update"), last_update, small, text_w))
	end

	local header = HorizontalGroup:new({
		align = "top",
		HorizontalSpan:new({ width = pad }),
		cover,
		HorizontalSpan:new({ width = pad }),
		facts,
	})

	-- Tags as chips.
	local tag_group
	local tags = Format.tagList(series.tags)
	if #tags > 0 then
		tag_group = HorizontalGroup:new({
			HorizontalSpan:new({ width = pad }),
			Widgets.chips(tags, inner_w, Font:getFace("xx_smallinfofont"), 3),
		})
	end

	-- Actions.
	-- local latest_selection, latest_count = Format.latest(total, 5)
	local actions = ButtonTable:new({
		width = inner_w,
		show_parent = self,
		buttons = {
			{
				{
					text = _("Choose chapters"),
					callback = function()
						if self.on_choose then
							self.on_choose()
						end
					end,
				},
				{
					text = _("Enter numbers…"),
					callback = function()
						if self.on_enter_numbers then
							self.on_enter_numbers()
						end
					end,
				},
			},
			{
				{
					text = _("First…"),
					enabled = total > 0,
					callback = function()
						if self.on_quick then
							self.on_quick("first")
						end
					end,
				},
				{
					text = _("Latest…"),
					enabled = total > 0,
					callback = function()
						if self.on_quick then
							self.on_quick("latest")
						end
					end,
				},
			},
			{
				{
					text = T(_("Download all (%1)"), total),
					enabled = total > 0,
					callback = function()
						if self.on_download then
							self.on_download("1-" .. total, total)
						end
					end,
				},
			},
		},
	})

	-- NOTE: VerticalGroup caches its size and child offsets the first time
	-- getSize() is called, so a group must never be measured and then have
	-- children added. `top` is finished before it is measured; `body` is only
	-- ever measured by the painter, after it is complete.
	local top = VerticalGroup:new({ align = "left", title_bar })
	table.insert(top, VerticalSpan:new({ width = pad }))
	table.insert(top, header)
	if tag_group then
		table.insert(top, VerticalSpan:new({ width = gap }))
		table.insert(top, tag_group)
	end
	table.insert(top, VerticalSpan:new({ width = pad }))
	table.insert(top, HorizontalGroup:new({ HorizontalSpan:new({ width = pad }), actions }))
	table.insert(top, VerticalSpan:new({ width = pad }))

	-- Synopsis fills whatever height is left and scrolls when it is longer.
	local synopsis_label = TextWidget:new({
		text = _("Synopsis"),
		face = Font:getFace("cfont", 18),
		bold = true,
		max_width = inner_w,
	})
	table.insert(top, HorizontalGroup:new({ HorizontalSpan:new({ width = pad }), synopsis_label }))
	table.insert(top, VerticalSpan:new({ width = gap }))

	local used = top:getSize().h -- `top` is complete from here on
	local synopsis_h = math.max(self.height - used - pad, Screen:scaleBySize(80))
	local description = Format.orElse(series.description, _("No synopsis available."))
	local synopsis = HorizontalGroup:new({
		HorizontalSpan:new({ width = pad }),
		ScrollTextWidget:new({
			text = description,
			face = Font:getFace("x_smallinfofont", 17),
			width = inner_w,
			height = synopsis_h,
			dialog = self,
		}),
	})
	local body = VerticalGroup:new({ align = "left", top, synopsis })

	self[1] = FrameContainer:new({
		background = Blitbuffer.COLOR_WHITE,
		bordersize = 0,
		padding = 0,
		margin = 0,
		body,
	})
end

function DetailView:onClose()
	UIManager:close(self)
	if self.on_close then
		self.on_close()
	end
	return true
end

function DetailView:onCloseWidget()
	UIManager:setDirty(nil, function()
		return "ui", self.dimen
	end)
end

return DetailView
