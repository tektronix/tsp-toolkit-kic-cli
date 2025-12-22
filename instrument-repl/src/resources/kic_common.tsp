if not _KIC then _KIC = {} end

_KIC["version"] = "!<!<VERSION>!>!"
_KIC["TSP_VERSIONS"] = { "tti", "2600", "mp5000" }
_KIC[".load_time_prompts"] = localnode.prompts

local _2600 = 0
local _3700 = 1
local TTI = 2
local MP5000 = 3

local models = {
    ["2450"] = TTI,
    ["2470"] = TTI,
    ["DMM7510"] = TTI,
    ["2460"] = TTI,
    ["2461"] = TTI,
    ["2461-SYS"] = TTI,
    ["DMM7512"] = TTI,
    ["DMM6500"] = TTI,
    ["DAQ6510"] = TTI,
    ["2601"] = _2600,
    ["2602"] = _2600,
    ["2611"] = _2600,
    ["2612"] = _2600,
    ["2635"] = _2600,
    ["2636"] = _2600,
    ["2601A"] = _2600,
    ["2602A"] = _2600,
    ["2611A"] = _2600,
    ["2612A"] = _2600,
    ["2635A"] = _2600,
    ["2636A"] = _2600,
    ["2651A"] = _2600,
    ["2657A"] = _2600,
    ["2601B"] = _2600,
    ["2601B-PULSE"] = _2600,
    ["2602B"] = _2600,
    ["2606B"] = _2600,
    ["2611B"] = _2600,
    ["2612B"] = _2600,
    ["2635B"] = _2600,
    ["2636B"] = _2600,
    ["2604B"] = _2600,
    ["2614B"] = _2600,
    ["2634B"] = _2600,
    ["2601B-L"] = _2600,
    ["2602B-L"] = _2600,
    ["2611B-L"] = _2600,
    ["2612B-L"] = _2600,
    ["2635B-L"] = _2600,
    ["2636B-L"] = _2600,
    ["2604B-L"] = _2600,
    ["2614B-L"] = _2600,
    ["2634B-L"] = _2600,
    ["3706"] = _3700,
    ["3706-SNFP"] = _3700,
    ["3706-S"] = _3700,
    ["3706-NFP"] = _3700,
    ["3706A"] = _3700,
    ["3706A-SNFP"] = _3700,
    ["3706A-S"] = _3700,
    ["3706A-NFP"] = _3700,
    ["707B"] = _3700,
    ["708B"] = _3700,
    ["5880-SRU"] = _3700,
    ["5881-SRU"] = _3700,
    ["VERSATEST-600"] = MP5000,
    ["TSPop"] = MP5000,
    ["TSP"] = MP5000,
    ["MP5103"] = MP5000,
}

_KIC["is_tti"] = function() return models[localnode.model] == TTI end
_KIC["is_2600"] = function() return models[localnode.model] == _2600 end
_KIC["is_3700"] = function() return models[localnode.model] == _3700 end
_KIC["is_mp5000"] = function() return models[localnode.model] == MP5000 end

---interate over input string and escape special characters in a given string,
---making it safe for inclusion in JSON
---@param s string
---@return string
local function escape_str(s)
    local in_char  = { '\\', '"', '/', '\b', '\f', '\n', '\r', '\t' }
    local out_char = { '\\', '"', '/', 'b', 'f', 'n', 'r', 't' }
    for i, c in ipairs(in_char) do
        s = string.gsub(s, c, '\\' .. out_char[i])
    end
    return s
end
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

if _KIC.is_tti() or _KIC.is_3700() then
    _KIC["error_message"] = function()
        local errorstr = [[ERM>START]] .. '\n'
        for _err_num = 1, eventlog.getcount(eventlog.SEV_ERROR) do
            local event_id, message, severity, node_id, secs, nanos = eventlog.next(eventlog.SEV_ERROR)
            errorstr = errorstr ..
                [[ERM>]] .. _KIC.toJson({
                    error_code = event_id,
                    message = escape_str(message),
                    severity = severity,
                    node_id = node_id,
                    time = { secs = secs, nanos = nanos }
                }) .. "\n"
        end
        errorstr = errorstr .. [[ERM>DONE]]
        return errorstr
    end
elseif _KIC.is_2600() then
    _KIC["error_message"] = function()
        local errorstr = [[ERM>START]] .. "\n"
        for _err_num = 1, errorqueue.count do
            local error_code, message, severity, node_id = errorqueue.next()
            errorstr = errorstr ..
            "ERM>" ..
            _KIC.toJson({ error_code = error_code, message = escape_str(message), severity = severity, node_id = node_id, time = nil }) ..
            "\n"
        end
        errorstr = errorstr .. [[ERM>DONE]]
        return errorstr
    end
elseif _KIC.is_mp5000() then
    _KIC["error_message"] = function()
        local errorstr = [[ERM>START]] .. "\n"
        for _err_num = 1, errorqueue.count do
            local error_code, message, severity, node_id = errorqueue.next()
            errorstr = errorstr ..
            "ERM>" ..
            _KIC.toJson({ error_code = error_code, message = escape_str(message), severity = severity, node_id = node_id, time = nil }) ..
            "\n"
        end
        errorstr = errorstr .. [[ERM>DONE]]
        return errorstr
    end
else
    -- Default function declarations just in case we encounter an unknown instrument
    -- model. Anything defined here should return a string that satisfies the
    -- caller so kic-cli can complete.
    _KIC["error_message"] = function()
        local errorstr = [[ERM>START]] .. "\n"
        for _err_num = 1, errorqueue.count do
            error_code = 0
            severity = 0
            node_id = 0
            message = "Model number not recognized: '" .. localnode.model .. "'"
            errorstr = errorstr ..
            "ERM>" ..
            _KIC.toJson({ error_code = error_code, message = escape_str(message), severity = severity, node_id = node_id, time = nil }) ..
            "\n"
        end
        errorstr = errorstr .. [[ERM>DONE]]
        return errorstr
    end
end

--- Print the given list of `fields` in the given list of `buffers` in a character delimited
--- table (CSV-style) format with the given `delimiter` used as the separator
---@param buffers string[] the list of buffers that should be printed
---@param fields string[] the list of buffer fields that should be printed
---@param delimiter string the string that should be printed to sepearate the values
_KIC["print_buffers_csv"] = function(buffers, fields, delimiter)
    local data_string = ""
    for i, buf in ipairs(buffers) do
        if string.len(data_string) ~= 0 then data_string = data_string .. "\n" end
        data_string = data_string .. "Buffer '" .. buf .. "'\n"
        if _G[buf] == nil or _G[buf].n == nil then
            data_string = data_string .. "DOES NOT EXIST\n"
        else
            local buffer = _G[buf]
            local header = "n"
            for j, f in ipairs(fields) do
                if buffer[f] ~= nil then
                    header = header .. delimiter .. f
                end
            end
            data_string = data_string .. header .. "\n"
            for r = 1, buffer.n do
                local row = tostring(r)
                for k, f in ipairs(fields) do
                    if buffer[f] ~= nil then
                        row = row .. delimiter .. tostring(buffer[f][r])
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

--clean up functions that aren't important after setup--
_KIC["is_2600"] = nil
_KIC["is_3700"] = nil
_KIC["is_tti"] = nil
_KIC["is_mp5000"] = nil
_KIC["set_tsp_version"] = nil
