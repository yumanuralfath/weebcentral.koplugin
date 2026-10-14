-- Pure helpers (no KOReader dependencies) so they can be unit tested.
local Format = {}

function Format.clean(value)
	value = tostring(value or "")
	return (value:gsub("^%s+", ""):gsub("%s+$", ""))
end

-- Returns `value` when it has content, otherwise `fallback`.
function Format.orElse(value, fallback)
	value = Format.clean(value)
	if value == "" then
		return fallback
	end
	return value
end

-- "2026-10-01T12:00:00.000Z" -> "2026-10-01"
function Format.isoDate(iso)
	return tostring(iso or ""):match("^(%d%d%d%d%-%d%d%-%d%d)") or ""
end

-- Newest publication date found in a chapter list (ISO strings sort lexicographically).
function Format.lastUpdate(chapters)
	local newest = ""
	for _, chapter in ipairs(chapters or {}) do
		local date = Format.isoDate(chapter.published_at)
		if date > newest then
			newest = date
		end
	end
	return newest
end

-- "Action, Adventure" -> { "Action", "Adventure" }
function Format.tagList(tags)
	local list = {}
	for tag in tostring(tags or ""):gmatch("[^,]+") do
		tag = Format.clean(tag)
		if tag ~= "" then
			list[#list + 1] = tag
		end
	end
	return list
end

-- Joins the first `limit` tags and reports how many were left out.
function Format.tagSummary(tags, limit, separator)
	local list = Format.tagList(tags)
	local shown = {}
	for index = 1, math.min(limit or #list, #list) do
		shown[index] = list[index]
	end
	return table.concat(shown, separator or " · "), math.max(#list - #shown, 0)
end

-- { [1]=true, [2]=true, [3]=true, [5]=true } -> "1-3,5"
function Format.compressSelection(selected)
	local indices = {}
	for index, on in pairs(selected or {}) do
		if on then
			indices[#indices + 1] = index
		end
	end
	table.sort(indices)

	local parts = {}
	local start, previous
	local function flush()
		if not start then
			return
		end
		parts[#parts + 1] = start == previous and tostring(start) or (start .. "-" .. previous)
	end
	for _, index in ipairs(indices) do
		if previous and index == previous + 1 then
			previous = index
		else
			flush()
			start, previous = index, index
		end
	end
	flush()
	return table.concat(parts, ",")
end

-- "1,3,5-7" -> { [1]=true, [3]=true, [5]=true, [6]=true, [7]=true }, 5
-- Returns nil when the text is invalid or outside 1..max.
function Format.parseSelection(text, max)
	local selected, count = {}, 0
	for part in tostring(text or ""):gmatch("[^,]+") do
		local first, last = part:match("^%s*(%d+)%s*%-%s*(%d+)%s*$")
		if not first then
			first = part:match("^%s*(%d+)%s*$")
			last = first
		end
		first, last = tonumber(first), tonumber(last)
		if not first or first < 1 or last < first or last > max then
			return nil
		end
		for index = first, last do
			if not selected[index] then
				selected[index] = true
				count = count + 1
			end
		end
	end
	if count == 0 then
		return nil
	end
	return selected, count
end

-- Selection string for the newest `amount` chapters (list is oldest -> newest).
function Format.latest(total, amount)
	if total < 1 then
		return nil, 0
	end
	local count = math.min(amount, total)
	local first = total - count + 1
	if first == total then
		return tostring(total), 1
	end
	return first .. "-" .. total, count
end

-- Selection string for the oldest `amount` chapters.
function Format.first(total, amount)
	if total < 1 then
		return nil, 0
	end
	local count = math.min(amount, total)
	if count == 1 then
		return "1", 1
	end
	return "1-" .. count, count
end

return Format
