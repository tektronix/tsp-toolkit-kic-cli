if not _KIC then _KIC = {} end

_KIC["version"] = "!<!<VERSION>!>!"
_KIC["TSP_VERSIONS"] = {"tti", "2600", "versatest"}
_KIC[".load_time_prompts"] = localnode.prompts

local _2600 = 0
local TTI = 1
local VERSATEST = 2

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
    ["3706"] = _2600,
    ["3706-SNFP"] = _2600,
    ["3706-S"] = _2600,
    ["3706-NFP"] = _2600,
    ["3706A"] = _2600,
    ["3706A-SNFP"] = _2600,
    ["3706A-S"] = _2600,
    ["3706A-NFP"] = _2600,
    ["707B"] = _2600,
    ["708B"] = _2600,
    ["VERSATEST-600"] = VERSATEST,
    ["TSPop"] = VERSATEST,
}

_KIC["is_tti"] = function() return models[localnode.model] == TTI end
_KIC["is_2600"] = function() return models[localnode.model] == _2600 end
_KIC["is_versatest"] = function() return models[localnode.model] == VERSATEST end

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

if _KIC.is_tti() then
    _KIC["error_message"] = function()
        local errorstr = [[ERM>START]] .. '\n'
        for _err_num = 1, eventlog.getcount(eventlog.SEV_ERROR) do
            local event_id, message, severity, node_id, secs, nanos = eventlog.next(eventlog.SEV_ERROR)
            if _err_num > 1 then errorstr = errorstr .. "," end
            errorstr = errorstr ..
                [[ERM>]] .. _KIC.toJson({
                    error_code = event_id,
                    message = message,
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
        local errorstr = [[ERM>START]].."\n"
        for _err_num = 1, errorqueue.count do
            local error_code, message, severity, node_id = errorqueue.next()
            errorstr = errorstr.. "ERM>" .._KIC.toJson({error_code=error_code, message=message, severity=severity, node_id=node_id, time=nil}) .. "\n"
        end
        errorstr = errorstr..[[ERM>DONE]]
        return errorstr
    end
elseif _KIC.is_versatest() then
    _KIC["error_message"] = function()
        local errorstr = [[ERM>START]].."\n"
        for _err_num = 1, errorqueue.count do
            local error_code, message, severity, node_id = errorqueue.next()
            errorstr = errorstr.. "ERM>" .._KIC.toJson({error_code=error_code, message=message, severity=severity, node_id=node_id, time=nil}) .. "\n"
        end
        errorstr = errorstr..[[ERM>DONE]]
        return errorstr
    end

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


_KIC["cleanup"] = function ()
    localnode.prompts = _KIC[".load_time_prompts"]
    _KIC = nil
end

--clean up functions that aren't important after setup--
_KIC["is_tti"] = nil
_KIC["is_2600"] = nil
_KIC["is_versatest"] = nil
_KIC["set_tsp_version"] = nil
