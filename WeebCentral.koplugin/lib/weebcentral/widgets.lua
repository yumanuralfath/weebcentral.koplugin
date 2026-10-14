-- Small reusable widgets: covers, badges, tag chips, separators.
local Blitbuffer = require("ffi/blitbuffer")
local CenterContainer = require("ui/widget/container/centercontainer")
local Device = require("device")
local Font = require("ui/font")
local FrameContainer = require("ui/widget/container/framecontainer")
local Geom = require("ui/geometry")
local HorizontalGroup = require("ui/widget/horizontalgroup")
local HorizontalSpan = require("ui/widget/horizontalspan")
local ImageWidget = require("ui/widget/imagewidget")
local LineWidget = require("ui/widget/linewidget")
local Size = require("ui/size")
local TextWidget = require("ui/widget/textwidget")
local VerticalGroup = require("ui/widget/verticalgroup")
local VerticalSpan = require("ui/widget/verticalspan")
local lfs = require("libs/libkoreader-lfs")
local _ = require("gettext")
local Screen = Device.screen

local Widgets = {}

function Widgets.lineHeight(face)
	local probe = TextWidget:new({ text = "Ag", face = face })
	local height = probe:getSize().h
	probe:free()
	return height
end

function Widgets.hasFile(path)
	return type(path) == "string" and path ~= "" and lfs.attributes(path, "mode") == "file"
end

-- A cover scaled to fit inside max_width x max_height, keeping its aspect
-- ratio. The returned widget is exactly as big as the image (plus border), so
-- callers should read its real size with :getSize(). Without an image a
-- placeholder of the full max size is returned.
function Widgets.cover(path, max_width, max_height)
	local border = Size.border.thin
	if Widgets.hasFile(path) then
		return FrameContainer:new({
			bordersize = border,
			padding = 0,
			margin = 0,
			ImageWidget:new({
				file = path,
				width = max_width - 2 * border,
				height = max_height - 2 * border,
				scale_factor = 0, -- best fit, keep aspect ratio
				file_do_cache = true,
				alpha = false,
			}),
		})
	end
	return FrameContainer:new({
		bordersize = border,
		padding = 0,
		margin = 0,
		width = max_width,
		height = max_height,
		background = Blitbuffer.COLOR_LIGHT_GRAY,
		CenterContainer:new({
			dimen = Geom:new({ w = max_width - 2 * border, h = max_height - 2 * border }),
			TextWidget:new({
				text = _("No cover"),
				face = Font:getFace("x_smallinfofont"),
				max_width = max_width - 2 * border - Screen:scaleBySize(4),
			}),
		}),
	})
end

-- Small rounded label. `inverted` draws white text on a black background.
function Widgets.badge(text, options)
	options = options or {}
	local inverted = options.inverted
	local pad_h = Screen:scaleBySize(6)
	local pad_v = Screen:scaleBySize(2)
	return FrameContainer:new({
		bordersize = Size.border.thin,
		radius = Size.radius.default,
		margin = 0,
		padding = 0,
		padding_left = pad_h,
		padding_right = pad_h,
		padding_top = pad_v,
		padding_bottom = pad_v,
		background = inverted and Blitbuffer.COLOR_BLACK or Blitbuffer.COLOR_WHITE,
		TextWidget:new({
			text = text,
			face = options.face or Font:getFace("x_smallinfofont"),
			bold = inverted or options.bold or false,
			fgcolor = inverted and Blitbuffer.COLOR_WHITE or Blitbuffer.COLOR_BLACK,
			max_width = options.max_width and (options.max_width - 2 * pad_h) or nil,
		}),
	})
end

-- Status label: solid for ongoing, outlined for everything else.
function Widgets.statusBadge(status, options)
	options = options or {}
	local lowered = tostring(status or ""):lower()
	options.inverted = lowered == "ongoing"
	return Widgets.badge(status, options)
end

-- Wraps tags into rows of badges that fit `max_width` (at most `max_rows`).
function Widgets.chips(tags, max_width, face, max_rows)
	local gap = Screen:scaleBySize(6)
	local group = VerticalGroup:new({ align = "left" })
	local row = HorizontalGroup:new({})
	local row_width, rows = 0, 1
	local hidden = 0

	for index, tag in ipairs(tags) do
		local chip = Widgets.badge(tag, { face = face, max_width = max_width })
		local width = chip:getSize().w
		if row_width > 0 and row_width + gap + width > max_width then
			if rows >= max_rows then
				hidden = #tags - index + 1
				break
			end
			table.insert(group, row)
			table.insert(group, VerticalSpan:new({ width = gap }))
			row = HorizontalGroup:new({})
			row_width = 0
			rows = rows + 1
		end
		if row_width > 0 then
			table.insert(row, HorizontalSpan:new({ width = gap }))
			row_width = row_width + gap
		end
		table.insert(row, chip)
		row_width = row_width + width
	end

	if hidden > 0 then
		local more = Widgets.badge("+" .. hidden, { face = face, bold = true })
		if row_width + gap + more:getSize().w <= max_width then
			table.insert(row, HorizontalSpan:new({ width = gap }))
			table.insert(row, more)
		end
	end
	table.insert(group, row)
	return group
end

function Widgets.separator(width)
	return LineWidget:new({
		dimen = Geom:new({ w = width, h = Size.line.thin }),
		background = Blitbuffer.COLOR_GRAY,
	})
end

return Widgets
