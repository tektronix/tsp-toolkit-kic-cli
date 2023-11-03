if not _KIC then _KIC = {} end
_KIC = {
    version = "!<!<VERSION>!>!",
    error_message = function()
        local errorstr = [[ERM>START]].."\n"
        for _err_num = 1, errorqueue.count do
            local error_code, message, severity, node_id = errorqueue.next()
            if _err_num > 1 then errorstr = errorstr .."," end
            errorstr = errorstr.. "ERM>" .._KIC.toJson({error_code=error_code, message=message, severity=severity, node_id=node_id, time=nil}) .. "\n"
        end
        errorstr = errorstr..[[ERM>DONE]]
    end,
    prompts_enable = function(enable)
        _G[".orig_prompts"] = localnode.prompts
        if(enable) then
            localnode.prompts = 0
        else
            localnode.prompts = 1
        end
    end,
    prompts_restore = function()
        localnode.prompts = _G[".orig_prompts"]
    end
}
