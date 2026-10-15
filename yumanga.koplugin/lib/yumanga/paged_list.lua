-- Full-screen list with a title bar, tappable rows and page navigation.
-- Subclasses provide `row_height` and `buildRow(item, index)`, and may
-- provide `buildActionBar()` for a button bar above the page navigation.
local Blitbuffer = require("ffi/blitbuffer")
local Button = require("ui/widget/button")
local CenterContainer = require("ui/widget/container/centercontainer")
local Device = require("device")
local FrameContainer = require("ui/widget/container/framecontainer")
local Geom = require("ui/geometry")
local GestureRange = require("ui/gesturerange")
local HorizontalGroup = require("ui/widget/horizontalgroup")
local HorizontalSpan = require("ui/widget/horizontalspan")
local InputContainer = require("ui/widget/container/inputcontainer")
local InputDialog = require("ui/widget/inputdialog")
local TitleBar = require("ui/widget/titlebar")
local UIManager = require("ui/uimanager")
local TopContainer = require("ui/widget/container/topcontainer")
local VerticalGroup = require("ui/widget/verticalgroup")
local T = require("ffi/util").template
local _ = require("gettext")
local Screen = Device.screen

-- A row that reports taps and long presses.
local Row = InputContainer:extend({
	width = nil,
	height = nil,
	content = nil,
	on_tap = nil,
	on_hold = nil,
})

function Row:init()
	self.dimen = Geom:new({ w = self.width, h = self.height })
	self.ges_events = {
		TapRow = { GestureRange:new({ ges = "tap", range = self.dimen }) },
		HoldRow = { GestureRange:new({ ges = "hold", range = self.dimen }) },
	}
	self[1] = self.content
end

function Row:onTapRow()
	if self.on_tap then
		self.on_tap()
	end
	return true
end

function Row:onHoldRow()
	if self.on_hold then
		self.on_hold()
	end
	return true
end

local PagedList = InputContainer:extend({
	covers_fullscreen = true, -- lets UIManager skip painting what is underneath
	title = "",
	subtitle = nil,
	items = nil,
	row_height = nil,
	page = 1,
	on_close = nil,
})
PagedList.Row = Row

function PagedList:init()
	self.width = Screen:getWidth()
	self.height = Screen:getHeight()
	self.dimen = Geom:new({ x = 0, y = 0, w = self.width, h = self.height })
	self.items = self.items or {}

	if Device:hasKeys() then
		self.key_events.Close = { { Device.input.group.Back } }
		self.key_events.NextPage = { { Device.input.group.PgFwd } }
		self.key_events.PrevPage = { { Device.input.group.PgBack } }
	end
	if Device:isTouchDevice() then
		self.ges_events.Swipe = {
			GestureRange:new({ ges = "swipe", range = self.dimen }),
		}
	end
	self:rebuild()
end

function PagedList:buildActionBar() end

function PagedList:buildPageNav(nav_height)
	local prev = Button:new({
		icon = "chevron.left",
		callback = function()
			self:onPrevPage()
		end,
		enabled = self.page > 1,
		bordersize = 0,
		show_parent = self,
	})
	local info = Button:new({
		text = T(_("Page %1 of %2"), self.page, self.page_count),
		callback = function()
			self:onGotoPage()
		end,
		bordersize = 0,
		show_parent = self,
	})
	local next_button = Button:new({
		icon = "chevron.right",
		callback = function()
			self:onNextPage()
		end,
		enabled = self.page < self.page_count,
		bordersize = 0,
		show_parent = self,
	})
	local gap = HorizontalSpan:new({ width = Screen:scaleBySize(24) })
	return CenterContainer:new({
		dimen = Geom:new({ w = self.width, h = nav_height }),
		HorizontalGroup:new({ prev, gap, info, gap, next_button }),
	})
end

function PagedList:rebuild()
	if self[1] then
		self[1]:free()
	end

	local title_bar = TitleBar:new({
		width = self.width,
		fullscreen = true,
		with_bottom_line = true,
		title = self.title,
		subtitle = self.subtitle,
		close_callback = function()
			self:onClose()
		end,
		show_parent = self,
	})
	local action_bar = self:buildActionBar()
	local nav_height = Screen:scaleBySize(56)
	local used = title_bar:getSize().h + nav_height
	if action_bar then
		used = used + action_bar:getSize().h
	end

	local list_height = self.height - used
	self.per_page = math.max(1, math.floor(list_height / self.row_height))
	self.page_count = math.max(1, math.ceil(#self.items / self.per_page))
	self.page = math.min(math.max(self.page, 1), self.page_count)

	local rows = VerticalGroup:new({ align = "left" })
	local first = (self.page - 1) * self.per_page + 1
	local last = math.min(first + self.per_page - 1, #self.items)
	for index = first, last do
		table.insert(rows, self:buildRow(self.items[index], index))
	end

	-- local body = VerticalGroup:new({ align = "left", title_bar })
	-- table.insert(
	-- 	body,
	-- 	FrameContainer:new({
	-- 		bordersize = 0,
	-- 		padding = 0,
	-- 		margin = 0,
	-- 		width = self.width,
	-- 		height = list_height,
	-- 		rows,
	-- 	})
	-- )

	local body = VerticalGroup:new({ align = "left", title_bar })
	-- Fixed-height slot: the action bar and page nav stay at the bottom even
	-- when the last page has fewer rows than the others.
	table.insert(
		body,
		TopContainer:new({
			dimen = Geom:new({ w = self.width, h = list_height }),
			rows,
		})
	)

	if action_bar then
		table.insert(body, action_bar)
	end
	table.insert(body, self:buildPageNav(nav_height))

	-- self[1] = FrameContainer:new({
	-- 	background = Blitbuffer.COLOR_WHITE,
	-- 	bordersize = 0,
	-- 	padding = 0,
	-- 	margin = 0,
	-- 	body,
	-- })

	self[1] = FrameContainer:new({
		background = Blitbuffer.COLOR_WHITE,
		bordersize = 0,
		padding = 0,
		margin = 0,
		width = self.width,
		height = self.height,
		body,
	})
end

function PagedList:refresh(refresh_type)
	self:rebuild()
	UIManager:setDirty(self, function()
		return refresh_type or "ui", self.dimen
	end)
end

function PagedList:setItems(items)
	self.items = items
	self.page = 1
	self:refresh()
end

function PagedList:goToPage(page)
	page = math.min(math.max(page, 1), self.page_count)
	if page ~= self.page then
		self.page = page
		self:refresh("partial")
	end
end

function PagedList:onNextPage()
	self:goToPage(self.page + 1)
	return true
end

function PagedList:onPrevPage()
	self:goToPage(self.page - 1)
	return true
end

function PagedList:onGotoPage()
	if self.page_count <= 1 then
		return true
	end
	local dialog
	dialog = InputDialog:new({
		title = _("Go to page"),
		input = tostring(self.page),
		input_type = "number",
		input_hint = T(_("1 - %1"), self.page_count),
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
					text = _("Go"),
					is_enter_default = true,
					callback = function()
						local page = tonumber(dialog:getInputText())
						UIManager:close(dialog)
						if page then
							self:goToPage(math.floor(page))
						end
					end,
				},
			},
		},
	})
	UIManager:show(dialog)
	dialog:onShowKeyboard()
	return true
end

function PagedList:onSwipe(_arg, ges)
	if ges.direction == "west" then
		return self:onNextPage()
	elseif ges.direction == "east" then
		return self:onPrevPage()
	elseif ges.direction ~= "north" and ges.direction ~= "south" then
		-- diagonal swipe: full refresh, like the stock menus
		UIManager:setDirty(nil, "full")
	end
	return true
end

function PagedList:onClose()
	UIManager:close(self)
	if self.on_close then
		self.on_close()
	end
	return true
end

function PagedList:onCloseWidget()
	UIManager:setDirty(nil, function()
		return "ui", self.dimen
	end)
end

return PagedList
