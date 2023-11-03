if not _KIC then _KIC = {} end
_KIC = {
    version = "!<!<VERSION>!>!",
    error_message = function()
        local errorstr = [[ERM>START]].."\n["
        for _err_num = 1, errorqueue.count do
            local error_code, message, severity, node_id = errorqueue.next()
            if _err_num > 1 then errorstr = errorstr .."," end
            errorstr = errorstr.._KIC.toJson({error_code=error_code, message=message, severity=severity, node_id=node_id, time=nil})
        end
        errorstr = errorstr.."\n]\n"..[[ERM>DONE]]
    end
}
