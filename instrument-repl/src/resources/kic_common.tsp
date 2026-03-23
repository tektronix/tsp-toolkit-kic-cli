_KIC = {}

_KIC["version"] = "!<!<VERSION>!>!"
_KIC[".load_time_prompts"] = localnode.prompts

--- Iterate over input string and escape special characters in a given string,
--- making it safe for inclusion in JSON
--- @param s string the string to escape
--- @return string json_escaped_string the json-compatible, escaped string
local function escape_str(s)
    if s == nil then return "" end
    local in_char  = { '\\', '"', '/', '\b', '\f', '\n', '\r', '\t' }
    local out_char = { '\\', '"', '/', 'b', 'f', 'n', 'r', 't' }
    for i, c in ipairs(in_char) do
        s = string.gsub(s, c, '\\' .. out_char[i])
    end
    return s
end

--- Encode a lua types as JSON (where possible)
--- @param o any The lua type to be encoded as JSON
--- @return string json The json-string encoding the provided lua type
_KIC["toJson"] = function(o)
    local s = ''
    if type(o) == 'table' then
        s = s .. '{'
        local obj_str = ''
        for k, v in pairs(o) do
            if string.len(obj_str) > 0 then
                obj_str = obj_str .. ','
            end
            local rhs = _KIC.toJson(v)
            obj_str = obj_str .. _KIC.toJson(k) .. ': ' .. rhs
        end
        s = s .. obj_str .. '}'
    elseif type(o) == "string" then
        s = s .. '"' .. o .. '"'
    elseif type(o) == "boolean" then
        if o then
            s = s .. "true"
        else
            s = s .. "false"
        end
    elseif type(o) == "nil" then
        s = s .. "null"
    elseif type(o) == "number" then
        s = s .. tostring(o)
    else
        s = s .. '"[[' .. type(o) .. ']]"'
    end
    return s
end

--- Stub implementation for if an errorqueue or eventlog are not found. This will
--- satisfy what TSP Toolkit is looking for so the user may continue on but will
--- also notify the user that we won't be able to get any error information.
--- @return string errors The string encoding the errors
_KIC["error_message"] = function()
    local errorstr = [[ERM>START]] .. '\n'
    errorstr = errorstr ..
        [[ERM>]] .. _KIC.toJson({
            error_code = -1000,
            message = escape_str(
                "TSP Toolkit: No eventlog or erroqueue found on instrument. Will not be able to fetch error information."),
            severity = 100,
            node_id = 1,
        }) .. "\n"
    errorstr = errorstr .. [[ERM>DONE]]
    return errorstr
end
--- Get the number of errors
---@type nil | fun(): number
_KIC["error_count"] = nil
--- Get the next error
---@type nil | fun(): number, string, number, number, number|nil, number|nil
_KIC["error_next"] = nil

if errorqueue ~= nil and errorqueue.count ~= nil and errorqueue.next ~= nil then
    _KIC["error_count"] = function() return errorqueue.count end
    _KIC["error_next"] = function() return errorqueue.next() end
elseif eventlog ~= nil and (eventlog.getcount ~= nil or eventlog.count ~= nil) and eventlog.next ~= nil then
    if eventlog.getcount ~= nil then
        _KIC["error_count"] = function() return eventlog.getcount(eventlog.SEV_ERROR) end
    elseif eventlog.count ~= nil then
        _KIC["error_count"] = function() return eventlog.count end
    end
    _KIC["error_next"] = function() eventlog.next(eventlog.SEV_ERROR) end
end

if _KIC["error_count"] ~= nil and _KIC["error_next"] ~= nil then
    --- Encode messages from the errorqueue in a standardized JSON and sentinel format
    --- for TSP Toolkit to read
    --- @return string errors The string encoding the errors
    _KIC["error_message"] = function()
        local errorstr = [[ERM>START]] .. '\n'
        for _err_num = 1, _KIC.error_count() do
            local event_id, message, severity, node_id, secs, nanos = _KIC.error_next()
            local error_tab = {
                error_code = event_id,
                message = escape_str(message),
                severity = severity,
                node_id = node_id,
            }
            if secs ~= nil then
                error_tab["time"] = { secs = secs }
                if nanos ~= nil then
                    error_tab["time"]["nanos"] = nanos
                end
            end
            errorstr = errorstr ..
                [[ERM>]] .. _KIC.toJson(error_tab) .. "\n"
        end
        errorstr = errorstr .. [[ERM>DONE]]
        return errorstr
    end
end


--- Print the given list of `fields` in the given list of `buffers` in a character delimited
--- table (CSV-style) format with the given `delimiter` used as the separator
---@param buffers {name:string, b: table|nil}|nil[] the list of buffer variables that should be printed
---@param fields string[] the list of buffer fields that should be printed
---@param delimiter string the string that should be printed to sepearate the values
_KIC["print_buffers_csv"] = function(buffers, fields, delimiter)
    local data_string = ""
    for i, buf in ipairs(buffers) do
        local success, value = pcall(loadstring("return " .. buf.name))
        if success then buf.b = value end
        if string.len(data_string) ~= 0 then data_string = data_string .. "\n" end
        data_string = data_string .. "Buffer '" .. buf.name .. "'\n"
        if buf.b == nil or buf.b.n == nil then
            data_string = data_string .. "DOES NOT EXIST\n"
        else
            local header = "n"
            for j, f in ipairs(fields) do
                if buf.b[f] ~= nil or string.find(f, "absolute_timestamps") ~= nil or string.find(f, "relative_timestamps") ~= nil then
                    header = header .. delimiter .. f
                end
            end
            data_string = data_string .. header .. "\n"
            for r = 1, buf.b.n do
                local row = tostring(r)
                for k, f in ipairs(fields) do
                    if string.find(f, "absolute_timestamps") ~= nil then
                        if buf.b.basetimestampseconds ~= nil then
                            row = row .. delimiter .. tostring(
                                buf.b.basetimestampseconds
                                + buf.b.basetimestampfractional
                                + buf.b.timestamps[r])
                        elseif buf.b.basetimestamp then
                            row = row .. delimiter .. tostring(
                                buf.b.basetimestamp
                                + buf.b.timestamps[r])
                        else
                            -- default to whatever timestamp we happen to have.
                            row = row .. delimiter .. tostring(buf.b.timestamps[r])
                        end
                    elseif string.find(f, "relative_timestamps") ~= nil then
                        if buf.b.relativetimestamps ~= nil then
                            row = row .. delimiter .. tostring(buf.b.relativetimestamps[r])
                        else
                            -- default to whatever timestamp we happen to have.
                            row = row .. delimiter .. tostring(buf.b.timestamps[r])
                        end
                    elseif buf.b[f] ~= nil then
                        row = row .. delimiter .. tostring(buf.b[f][r])
                    end
                end
                data_string = data_string .. row .. "\n"
            end
        end
    end

    print(data_string)
end

_KIC["prompts_enable"] = function(enable)
    _G[".orig_prompts"] = localnode.prompts
    if (enable) then
        localnode.prompts = 1
    else
        localnode.prompts = 0
    end
end
_KIC["prompts_restore"] = function()
    localnode.prompts = _G[".orig_prompts"]
end


_KIC["cleanup"] = function()
    localnode.prompts = _KIC[".load_time_prompts"]
    _KIC = nil
end
