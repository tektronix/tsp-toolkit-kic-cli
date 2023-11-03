if not _KIC then _KIC = {} end

_KIC["version"] = "!<!<VERSION>!>!"
_KIC["error_message"] = function()
    local errorstr = [[ERM>START]].. '\n'
    for _err_num = 1, eventlog.getcount(eventlog.SEV_ERROR) do
        local event_id, message, severity, node_id, secs, nanos = eventlog.next(eventlog.SEV_ERROR)
        if _err_num > 1 then errorstr = errorstr .. "," end
        errorstr = errorstr .. [[ERM>]] .. _KIC.toJson({ error_code = event_id, message = message, severity = severity, node_id = node_id,
           time = { secs = secs, nanos = nanos } }).."\n"
    end
    errorstr = errorstr .. [[ERM>DONE]]
    return errorstr
end
_KIC["prompts_enable"] = function(enable)
    _G[".orig_prompts"] = localnode.prompts
    if (enable) then
        localnode.prompts = localnode.ENABLE
    else
        localnode.prompts = localnode.DISABLE
    end
end
_KIC["prompts_restore"] = function()
    localnode.prompts = _G[".orig_prompts"]
end
