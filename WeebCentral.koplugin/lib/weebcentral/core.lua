local rapidjson = require("rapidjson")
local _ = require("gettext")

local Core = {}

local function shellQuote(value)
    return "'" .. value:gsub("'", "'\\''") .. "'"
end

function Core.run(plugin_directory, ...)
    if not plugin_directory then
        return nil, false
    end

    local command = shellQuote(plugin_directory .. "bin/weebcentral_core_dev")
    for _, argument in ipairs({ ... }) do
        command = command .. " " .. shellQuote(tostring(argument))
    end

    local pipe = io.popen(command .. " 2>&1", "r")
    if not pipe then
        return nil, false
    end

    local output = pipe:read("*a") or ""
    local succeeded = pipe:close()
    return output, succeeded == true
end

function Core.decodeResponse(output, succeeded)
    if not output then
        return nil, _("Could not run the WeebCentral core executable.")
    end

    local response, decode_error = rapidjson.decode(output)
    if not response then
        return nil, _("Invalid response from the core: ")
            .. tostring(decode_error)
            .. "\n\n"
            .. output
    end
    if not succeeded or response.ok ~= true then
        return nil, response.error or _("The WeebCentral operation failed.")
    end

    return response.data
end

return Core
