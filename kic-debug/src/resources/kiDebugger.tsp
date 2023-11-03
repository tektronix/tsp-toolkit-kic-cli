kiDebuggerVersion = { version = "0.9.1.2013.0520" };
local function deleteScript(funcinfo, defaultname)
    local scriptname = defaultname;
    local namesOfScriptsToDelete = {};
    for name, script in pairs(script.user.scripts) do
        if (script.run == funcinfo.func) or (script.name == scriptname) then
            scriptname = name;
            table.insert(namesOfScriptsToDelete, name);
        end
    end
    for name in namesOfScriptsToDelete do
    end
    _G[scriptname] = nil;
    for name in script.user.catalog() do
        if (name == scriptname) then
            script.user.delete(scriptname);
            break;
        end
    end
end

--deleteScript( debug.getinfo( 1 ), "kiDebugger" );
local tailCallSignature = "\t(tail call): ?\n";
local kiBreakpoints = {}
local kiWatchpoints = {}
local kiDebugState = "kiRun"
local kiBreakpointCount = 0
local kiWatchpointCount = 0
local kiStack = 3
local kiStackStepOut = 0
local kiStackStepOver = 0
local kiDebuggedSource
local kiDebuggedSourceName
local inDebugConsoleBlock = false;
local kiMessageFormat
local kiStackTraceStepOver = ""
local kiStackTraceStepOut = ""
local kiRunOnExit;
local tspdbg = _G['.tspdbg'];
local expressionVariables = {};
if (tspdbg) then
    kiSetTableToStringFunction = function(t)
        local mt = getmetatable(t);
        if (not (mt)) then
            mt = {};
            setmetatable(t, mt);
        end
        mt.__tostring = tspdbg.tabletostring;
    end;
end
local function openloggerfactory()
    local temp = os.getenv("tspdbgdir");
    if (temp) then
        return function()
            return io.open(temp .. "\\tspdbg.log", "a+");
        end;
    elseif (not os.getenv("windir") and fs and fs.is_dir and fs.is_dir("/usb1/tspdbg")) then
        return function()
            return io.open("/usb1/tspdbg/tspdbg.log", "a+");
        end;
    else
        return nil;
    end
end

local openlogger = openloggerfactory();
local function log(...)
    if (openlogger) then
        local logger = openlogger();
        logger:write(string.format("%s\t", os.date("%Y-%m-%d %H:%M:%S")));
        table.foreachi(arg, function(_, v) logger:write(tostring(v) .. "\t"); end);
        logger:write("\n");
        logger:close();
    end
end

log("--==[[kiDebugger loaded, version " .. kiDebuggerVersion.version .. "]]==--");
local function logRemoteComm(data)
    if (data) then
        if (kiLogger) then
            kiLogger:log(tostring(data));
        end
    end
end

local function pack(...)
    return arg;
end
---it splits string on a separator and returns a table with split items
---@param str string string to be split
---@param seperator string pattern to split string on
---@return table splitTable table with split items
function string:split(str, seperator)
    local stringIndex = 1
    local splitTable = {}
    while(true) do
        local startIndex, endIndex = string.find(str, seperator)
        if (startIndex == nil) then
            break
        end
        table.insert(splitTable, string.sub(str, stringIndex, startIndex - 1 ))
        str = string.sub(str, endIndex + 1)
    end
    table.insert(splitTable, str)
    return splitTable
end

local function kiSendMessage(text, elementName, ...)
    kiMessageFormat = kiMessageFormat or "";
    if (text ~= nil) then
        log(text);
    end
    if (elementName ~= nil) then
        log(elementName, unpack(arg));
    end
    if (kiMessageFormat ~= "xml") then
        if (elementName == nil) then
            print(text);
        end
    else
        if (elementName == nil) then
            if (text == "DEBUG~~ECHO DEBUG CONSOLE") then
                inDebugConsoleBlock = true;
            elseif (text == "DEBUG~~ECHO~~OFF DEBUG CONSOLE") then
                inDebugConsoleBlock = false;
            elseif (not (inDebugConsoleBlock)) then
                return;
            end
            elementName = "debug-text";
        end
        local function xmlEscape(s)
            local byteStrings = {}
            for i = 1, string.len(s) do
                local byte = string.byte(s, i);
                local byteString;
                if (byte > 127) then byteString = string.format("&#x02%d;", byte);
                elseif (byte < 32) then byteString = string.format("&#x24%02x;", byte);
                elseif (byte == 38) then byteString = "&amp;";
                elseif (byte == 34) then byteString = "&quot;";
                elseif (byte == 39) then byteString = "&apos;";
                elseif (byte == 60) then byteString = "&lt;";
                elseif (byte == 62) then byteString = "&gt;";
                else byteString = string.char(byte); end
                table.insert(byteStrings, byteString);
            end
            return table.concat(byteStrings);
        end

        local xml = {};
        table.insert(xml, "<tspdbg");
        table.insert(xml, "-AA4E9540-A46C-4671-81D7-4FE69A9B6DC4>");
        if (elementName) then
            table.insert(xml, "<");
            table.insert(xml, elementName);
            table.insert(xml, " ");
            local n = arg.n;
            arg[n + 1] = "nil";
            for i = 1, n, 2 do
                table.insert(xml, arg[i]);
                table.insert(xml, "='");
                table.insert(xml, xmlEscape(tostring(arg[i + 1])));
                table.insert(xml, "' ");
            end
            if (text) then
                table.insert(xml, ">");
                table.insert(xml, xmlEscape(text));
                table.insert(xml, "</");
                table.insert(xml, elementName);
                table.insert(xml, ">");
            else
                table.insert(xml, "/>");
            end
        elseif (text) then
            table.insert(xml, xmlEscape(text));
        end
        table.insert(xml, "</tspdbg");
        table.insert(xml, "-AA4E9540-A46C-4671-81D7-4FE69A9B6DC4>");
        print(table.concat(xml));
    end
end

local function stringendswith(text, suffix)
    return string.sub(text, -string.len(suffix)) == suffix;
end

local function partitionlines(s)
    local function split(s, pat)
        local st, g = 1, string.gfind(s, "()(" .. pat .. ")");
        local function getter(self, segs, seps, sep, cap1, ...)
            st = sep and seps + string.len(sep);
            return string.sub(s, segs, (seps or 0) - 1), cap1 or sep, unpack(arg);
        end

        local function splitter(self)
            if st then return getter(self, st, g()); end
        end

        return splitter;
    end

    local t = {};
    for line, delim in split(s, "\n") do
        table.insert(t, line .. (delim or ""));
    end
    return t;
end

local function trimstack(stack, firstlinetodelete, lastlinetodelete)
    local lines = partitionlines(stack);
    for i = lastlinetodelete, firstlinetodelete, -1 do
        table.remove(lines, i);
    end
    return table.concat(lines);
end

local function aborthook()
    if ki.aborthook then
        ki.aborthook();
    end
end

local messageBuffer = "";
local function getmessage()
    aborthook();
    local message = nil;
    local data = ki.remotecomm.getmessage();
    if (data) then
        logRemoteComm(data);
        messageBuffer = messageBuffer .. data;
    end
    if (messageBuffer) then
        local newLineIndex = string.find(messageBuffer, "\n", 1, 1);
        if (newLineIndex) then
            message = string.sub(messageBuffer, 1, newLineIndex - 1);
            messageBuffer = string.sub(messageBuffer, newLineIndex + 1);
        end
    end
    return message;
end

local function scpiparamtonumber(text)
    local value;
    local leadingExcess;
    local trailingExcess;
    local _;
    local hexadecimal;
    _, _, leadingExcess, hexadecimal, trailingExcess = string.find(text, "([^#]*)#[hH]([0-9a-hA-H]+)([^0-9a-hA-H]*)");
    if (hexadecimal) then
        value = tonumber(hexadecimal, 16);
    else
        local octal;
        _, _, leadingExcess, octal, trailingExcess = string.find(text, "([^#]*)#[oO]([0-8]+)([^0-8]*)");
        if (octal) then
            value = tonumber(octal, 8);
        else
            local binary;
            _, _, leadingExcess, binary, trailingExcess = string.find(text, "([^#]*)#[bB]([0-1]+)([^0-1]*)");
            if (binary) then
                value = tonumber(binary, 2);
            else
                local decimal;
                _, _, leadingExcess, decimal, trailingExcess = string.find(text, "([^0-9]*)([0-9]+)([^0-9]*)");
                if (decimal) then
                    value = tonumber(decimal, 10);
                else
                    value = 0;
                end
            end
        end
    end
    if (leadingExcess and string.find(leadingExcess, "[^ ]")) then
        value = 0;
    elseif (trailingExcess and string.find(trailingExcess, "[^ ]")) then
        value = 0;
    end
    return value;
end

local function doscpi(msg)
    local upperMsg = string.upper(msg);
    if upperMsg == "*CLS" then
        log("SCPI: " .. msg);
        status.reset();
    elseif upperMsg == "*ESE?" then
        log("SCPI: " .. msg);
        return tostring(status.standard.enable);
    elseif string.sub(upperMsg, 1, 5) == "*ESE " then
        log("SCPI: " .. msg);
        local _, _, mask = string.sub(msg, 6);
        status.standard.enable = scpiparamtonumber(mask);
    elseif upperMsg == "*ESR?" then
        log("SCPI: " .. msg);
        return tostring(status.standard.event);
    elseif upperMsg == "*IDN?" then
        log("SCPI: " .. msg);
        return string.format("%s, Model %s, %s, %s", "Keithley Instruments Inc", localnode.model, localnode.serialno,
            localnode.revision);
    elseif upperMsg == "*OPC?" then
        log("SCPI: " .. msg);
        waitcomplete();
        return "1";
    elseif upperMsg == "*OPC" then
        log("SCPI: " .. msg);
        opc();
    elseif upperMsg == "*RST" then
        log("SCPI: " .. msg);
        reset();
    elseif upperMsg == "*SRE?" then
        log("SCPI: " .. msg);
        return tostring(status.request_enable);
    elseif string.sub(upperMsg, 1, 5) == "*SRE " then
        log("SCPI: " .. msg);
        local _, _, mask = string.sub(msg, 6);
        status.request_enable = scpiparamtonumber(mask);
    elseif upperMsg == "*STB?" then
        log("SCPI: " .. msg);
        return tostring(status.condition);
    elseif upperMsg == "*TRG" then
        log("SCPI ignored: " .. msg);
    elseif upperMsg == "*WAI" then
        log("SCPI: " .. msg);
        waitcomplete();
    else
        return "-285, TSP Syntax error at line 1: unexpected symbol near `*'";
    end
    return nil;
end

local function updateExpressionVariables()
    expressionVariables = {};
    for i = 1, kiBreakpointCount do
        local breakpoint = kiBreakpoints[i]
        if (breakpoint[2] and breakpoint[4]) then
            for name in pairs(breakpoint[4].variables) do
                expressionVariables[name] = true;
            end
        end
    end
    for i = 1, kiWatchpointCount do
        local watchpoint = kiWatchpoints[i]
        if (watchpoint.enabled) then
            for name in pairs(watchpoint.variables) do
                expressionVariables[name] = true;
            end
        end
    end
end

function kiSetBreakpoint(line, enabled, conditional)
    log("kiSetBreakpoint(" .. tostring(line) .. "," .. tostring(enabled) ..
        "," .. string.format("%q", conditional or "") .. ")");
    local index = kiBreakpointCount + 1
    local breakpointCollection =
    {
        tonumber(line),
        tonumber(enabled),
        conditional,
        conditional and tspdbg:compileExpression(conditional)
    }
    kiBreakpoints[index] = breakpointCollection
    kiBreakpointCount = index
    updateExpressionVariables();
end

function kiClearBreakpoints()
    log("kiClearBreakpoints");
    kiBreakpointCount = 0
    kiBreakpoints = {}
end

function kiSetWatchpoint(source, enabled)
    local watchpoint = tspdbg:compileExpression(source)
    watchpoint.enabled = (enabled == 1);
    local variables = { "variables: " };
    for variable in pairs(watchpoint.variables) do table.insert(variables, variable); end
    log("kiSetWatchpoint(" .. tostring(source) .. "," .. tostring(enabled) .. ")",
        watchpoint.compilationError or "syntax: ok", table.concat(variables, " "));
    kiClearWatchpoint(source)
    local index = kiWatchpointCount + 1
    kiWatchpoints[index] = watchpoint
    kiWatchpointCount = index
    updateExpressionVariables();
    if (kiMessageFormat == "xml") then
        tspdbg.expressionVariables = expressionVariables
        tspdbg.watchpoints = kiWatchpoints
        tspdbg:sendstacktrace(7);
    end
end

function kiClearWatchpoints()
    log("kiClearWatchpoints");
    kiWatchpointCount = 0
    kiWatchpoints = {}
end

function kiClearWatchpoint(expression)
    for index = kiWatchpointCount, 1, -1 do
        local watchpoint = kiWatchpoints[index]
        if (expression == watchpoint[1]) then
            local lastIndex = kiWatchpointCount
            kiWatchpoints[lastIndex], kiWatchpoints[index] = kiWatchpoints[index], kiWatchpoints[lastIndex]
            kiWatchpointCount = kiWatchpointCount - 1
            kiWatchpoints[lastIndex] = nil
        end
    end
end

local function kiDebugAck()
    kiSendMessage("DEBUG~~ACKNOWLEDGE_OK")
end

function string.starts(String, Start)
    return string.sub(String, 1, string.len(Start)) == Start
end

local function remoteCommInput()
    local kiLoopState = 0
    ki.remotecomm.intercept = 1
    local sendPromptForNewCommand = true;
    while true do
        if kiDebugState == "kiRun" then
            break
        end
        if (sendPromptForNewCommand) then
            kiSendMessage(nil, "debug-prompt");
            sendPromptForNewCommand = false;
        end
        local msg = getmessage()
        if (msg == nil) then
            delay(.005)
        elseif string.starts(msg, "kiRun") then
            ki.remotecomm.intercept = 0;
            kiLoopState = 1
            kiDebugState = "kiRun"
            kiSendMessage("DEBUG~~ECHO DEBUG CONSOLE")
            kiSendMessage("Application running...")
            kiSendMessage("DEBUG~~ECHO~~OFF DEBUG CONSOLE")
            kiSendMessage(nil, "resume-run");
        elseif string.starts(msg, "kiStepIn") then
            ki.remotecomm.intercept = 0;
            kiLoopState = 1
            kiDebugState = "kiStepIn"
            kiSendMessage(nil, "resume-stepin");
        elseif string.starts(msg, "kiStepOut") then
            ki.remotecomm.intercept = 0
            kiLoopState = 1
            kiDebugState = "kiStepOut"
            kiSendMessage(nil, "resume-stepout");
            kiStackStepOut = kiStack
            kiStackTraceStepOut = trimstack(debug.traceback(), 1, 5);
        elseif string.starts(msg, "kiStepOver") then
            ki.remotecomm.intercept = 0
            kiLoopState = 1
            kiDebugState = "kiStepOver"
            kiSendMessage(nil, "resume-stepover");
            kiStackStepOver = kiStack
            kiStackTraceStepOver = trimstack(debug.traceback(), 1, 4)
        elseif string.starts(msg, "kiSetBreakpoints") then
            kiDebugAck()
            kiClearBreakpoints()
            while true do
                msg = getmessage()
                if (msg == nil) then
                    delay(.005)
                elseif ((string.lower(msg) == "abort") or (string.sub(string.lower(msg), 1, 6) == "login ")) then
                    if (string.sub(string.lower(msg), 1, 6) == "login ") then
                        print("FAILURE: The debugger is active and it cannot proccess the LOGIN command. Aborting debugging. Try LOGIN again.");
                    end
                    ki.remotecomm.intercept = 0
                    exit();
                elseif (string.sub(msg, 1, 1) == "*") then
                    local output = doscpi(msg);
                    if (output) then
                        print(output);
                    end
                else
                    kiDebugAck()
                    if msg == "kiSetBreakpointsDone" then
                        break;
                    end
                    local delimiterPosition = string.find(msg, ",")
                    if delimiterPosition then
                        local bpLine = tonumber(string.sub(msg, 1, delimiterPosition - 1));
                        local delimiterPosition1 = string.find(msg, ",", delimiterPosition + 1) or 0;
                        local bpEnabled = tonumber(string.sub(msg, delimiterPosition + 1, delimiterPosition1 - 1));
                        local bpConditional = delimiterPosition1 > 0 and string.sub(msg, delimiterPosition1 + 1) or nil;
                        kiSetBreakpoint(bpLine, bpEnabled, bpConditional);
                    end
                end
            end
            sendPromptForNewCommand = true;
        elseif string.starts(msg, "kiSetWatchpoints") then
            kiDebugAck()
            kiClearWatchpoints()
            while true do
                msg = getmessage()
                if (msg == nil) then
                    delay(.005)
                elseif ((string.lower(msg) == "abort") or (string.sub(string.lower(msg), 1, 6) == "login ")) then
                    if (string.sub(string.lower(msg), 1, 6) == "login ") then
                        print("FAILURE: The debugger is active and it cannot proccess the LOGIN command. Aborting debugging. Try LOGIN again.");
                    end
                    ki.remotecomm.intercept = 0
                    exit();
                elseif (string.sub(msg, 1, 1) == "*") then
                    local output = doscpi(msg);
                    if (output) then
                        print(output);
                    end
                else
                    kiDebugAck()
                    if msg == "kiSetWatchpointsDone" then
                        break;
                    end
                    local delimiterPosition = string.find(msg, ",")
                    if delimiterPosition then
                        local wpExpression = string.sub(msg, 1, delimiterPosition - 1)
                        local wpEnabled = tonumber(string.sub(msg, delimiterPosition + 1))
                        kiSetWatchpoint(wpExpression, wpEnabled)
                    end
                end
            end
            sendPromptForNewCommand = true;
        elseif string.starts(msg, "kiExecuteFromConsole") then
            kiDebugAck()
            while true do
                msg = getmessage()
                if (msg == nil) then
                    delay(.005)
                elseif ((string.lower(msg) == "abort") or (string.sub(string.lower(msg), 1, 6) == "login ")) then
                    if (string.sub(string.lower(msg), 1, 6) == "login ") then
                        print("FAILURE: The debugger is active and it cannot proccess the LOGIN command. Aborting debugging. Try LOGIN again.");
                    end
                    ki.remotecomm.intercept = 0
                    exit();
                elseif (string.sub(msg, 1, 1) == "*") then
                    local output = doscpi(msg);
                    if (output) then
                        print(output);
                    end
                else
                    kiDebugAck()
                    if msg == "kiExecuteFromConsoleDone" then
                        break;
                    end
                    local cmdExe, err = loadstring(msg)
                    if err then
                        kiSendMessage("Debug Mode Error: You have entered a console statement that does not compile. Check syntax and try again.\n"
                            .. err)
                        kiSendMessage("TSP?")
                    else
                        local _, err = pcall(cmdExe)
                        if err then
                            kiSendMessage("Debug Mode Error: You have entered a console statement that has generated an error. Check your statements and try again. \n"
                                .. err)
                            kiSendMessage("TSP?")
                        else
                            kiSendMessage("TSP>")
                        end
                    end
                end
            end
        elseif (string.starts(msg, "abort") or (string.sub(string.lower(msg), 1, 6) == "login ")) then
            log(msg);
            if (string.sub(string.lower(msg), 1, 6) == "login ") then
                print("FAILURE: The debugger is active and it cannot proccess the LOGIN command. Aborting debugging. Try LOGIN again.");
            end
            ki.remotecomm.intercept = 0;
            kiLoopState = 1;
            if ((msg == "ABORT") or (string.sub(string.lower(msg), 1, 6) == "login ")) then
                kiDebugState = "kiShellAbort";
            else
                kiDebugState = "kiAbortScript";
            end
            kiRunOnExit(
                function()
                end
            );
        elseif (string.sub(msg, 1, 1) == "*") then
            local output = doscpi(msg);
            if (output) then
                print(output);
            end
        else
            log("command: " .. msg);
            local cmdLineFunc, compilationError = loadstring(msg);
            if (cmdLineFunc) then
                local results = pack(xpcall(cmdLineFunc, debug.traceback));
                local runOk = table.remove(results, 1);
                if (not (runOk)) then
                    kiSendMessage(nil, "command-execution-failed", "error", results[1]);
                end
            else
                kiSendMessage(nil, "command-compilation-failed", "error", compilationError, "source", msg);
            end
            sendPromptForNewCommand = true;
        end
        if kiLoopState == 1 then
            break
        end
    end
end

local function kiDebug()
    if kiDebugState == "kiBreak" then
        remoteCommInput()
    elseif kiDebugState == "kiStepIn" then
        remoteCommInput()
    elseif kiDebugState == "kiStepOut" then
        remoteCommInput()
    elseif kiDebugState == "kiStepOver" then
        remoteCommInput()
    elseif kiDebugState == "kiRun" then
        kiSendMessage("DEBUG~~ECHO DEBUG CONSOLE")
        kiSendMessage("Application running...")
        kiSendMessage("DEBUG~~ECHO~~OFF DEBUG CONSOLE")
    else
        error("Unknown kiDebugState: " .. tostring(kiDebugState));
    end
    if ((kiDebugState == "kiAbortScript") or (kiDebugState == "kiShellAbort")) then
        debug.sethook()
        error("Script aborted", 3);
    end
end

local function isEtherFunction(debuginfo)
    return kiDebuggedSourceName ~= debuginfo.source and kiDebuggedSource ~= debuginfo.source;
end

local function kiDebugHook(kiEvent, kiLine)
    local kiDebugVariable
    local kiResumeDebug = 0
    local kiEvalVars = 0
    local env;
    aborthook();
    if kiDebugState == "kiStepOver" then
        if (kiStack - 1) <= kiStackStepOver then
            local kiStackTrace = trimstack(debug.traceback(), 1, 3);
            local tailCallIsAtTop = string.sub(kiStackTrace, 1, string.len(tailCallSignature)) == tailCallSignature;
            if (tailCallIsAtTop and not (stringendswith(kiStackTrace, kiStackTraceStepOver))) then
                kiStackTraceStepOver = tailCallSignature .. trimstack(kiStackTraceStepOver, 1, 1);
            end
            if (not (stringendswith(kiStackTrace, kiStackTraceStepOver))) then
                kiDebugState = "kiStepOverReached"
            end
        end
    elseif kiDebugState == "kiStepOut" then
        if kiStack <= kiStackStepOut then
            local kiStackTrace = trimstack(debug.traceback(), 1, 3)
            if (not (stringendswith(kiStackTrace, kiStackTraceStepOut))) then
                kiDebugState = "kiStepOutReached"
            end
        end
    end
    if kiEvent == "call" then
        kiStack = kiStack + 1
        local kiDebugInfo = debug.getinfo(2)
        assert((kiDebugInfo.name ~= "exit") or (kiDebugInfo.what ~= "C"), "exit called by user code");
        if kiDebugState == "kiRun" then
        elseif kiDebugState == "kiStepIn" then
        elseif kiDebugState == "kiStepOut" then
        elseif kiDebugState == "kiStepOver" then
        end
    elseif kiEvent == "return" or kiEvent == "tail return" then
        if kiDebugState == "kiRun" then
        elseif kiDebugState == "kiStepIn" then
        elseif kiDebugState == "kiStepOut" then
            if kiStack <= kiStackStepOut then
                local kiStackTrace = trimstack(debug.traceback(), 1, 3)
                if (not (stringendswith(kiStackTrace, kiStackTraceStepOut))) then
                    kiDebugState = "kiStepOutReached"
                end
            end
        elseif kiDebugState == "kiStepOver" then
            if (kiStack - 1) <= kiStackStepOver then
                local kiStackTrace = trimstack(debug.traceback(), 1, 3)
                if (not (stringendswith(kiStackTrace, kiStackTraceStepOver))) then
                    kiDebugState = "kiStepOverReached"
                end
            end
        end
        kiStack = kiStack - 1
    elseif kiEvent == "line" then
        local breakpointResult, condition = false, nil;
        for i = 1, kiBreakpointCount do
            local bpCollection = kiBreakpoints[i]
            if (bpCollection[1] == kiLine) then
                if (bpCollection[2] == 1) then
                    if (bpCollection[3]) then
                        breakpointResult, condition = bpCollection[3], bpCollection[4];
                    else
                        breakpointResult, condition = true, nil;
                    end
                else
                    breakpointResult, condition = false, nil;
                end
                break;
            end
        end
        if (condition or (kiWatchpointCount > 0)) then
            env = tspdbg:createExpressionEnvironment(expressionVariables, 2);
        end
        local breakpointHit;
        if (condition) then
            local success, value = condition:evaluateInEnvironment(env);
            breakpointHit = success and value;
        else
            breakpointHit = breakpointResult;
        end
        if (breakpointHit) then
            if (not (isEtherFunction(debug.getinfo(2)))) then
                kiDebugVariable = nil
                kiResumeDebug = 1
                kiDebugState = "kiBreak"
                kiSendMessage(nil, "breakpoint", "line", kiLine);
                kiSendMessage("DEBUG~~ECHO DEBUG CONSOLE")
                kiSendMessage("Application hit breakpoint at line " .. kiLine)
                kiSendMessage("DEBUG~~ECHO~~OFF DEBUG CONSOLE")
                kiSendMessage("DEBUG~~ECHO HIT BREAKPOINT")
                kiSendMessage(kiLine)
                kiSendMessage("DEBUG~~ECHO~~OFF HIT BREAKPOINT")
                kiEvalVars = 1
            end
        end
        if kiDebugState == "kiRun" then
        elseif kiDebugState == "kiStepIn" then
            if isEtherFunction(debug.getinfo(2)) then
                kiResumeDebug = 0
            else
                kiSendMessage(nil, "suspend-stepin", "line", kiLine);
                kiSendMessage("DEBUG~~ECHO DEBUG CONSOLE")
                kiSendMessage("Application single stepped to line " .. kiLine)
                kiSendMessage("DEBUG~~ECHO~~OFF DEBUG CONSOLE")
                kiSendMessage("DEBUG~~ECHO HIT BREAKPOINT")
                kiSendMessage(kiLine)
                kiSendMessage("DEBUG~~ECHO~~OFF HIT BREAKPOINT")
                kiEvalVars = 1
                kiResumeDebug = 1
            end
        elseif kiDebugState == "kiStepOutReached" then
            if isEtherFunction(debug.getinfo(2)) then
                kiResumeDebug = 0
            else
                kiSendMessage(nil, "suspend-stepout", "line", kiLine);
                kiSendMessage("DEBUG~~ECHO DEBUG CONSOLE")
                kiSendMessage("Application stepped out of function to line " .. kiLine)
                kiSendMessage("DEBUG~~ECHO~~OFF DEBUG CONSOLE")
                kiSendMessage("DEBUG~~ECHO HIT BREAKPOINT")
                kiSendMessage(kiLine)
                kiSendMessage("DEBUG~~ECHO~~OFF HIT BREAKPOINT")
                kiDebugState = "kiStepIn"
                kiEvalVars = 1
                kiResumeDebug = 1
            end
        elseif kiDebugState == "kiStepOverReached" then
            if isEtherFunction(debug.getinfo(2)) then
                kiResumeDebug = 0
            else
                kiSendMessage(nil, "suspend-stepover", "line", kiLine);
                kiSendMessage("DEBUG~~ECHO DEBUG CONSOLE")
                kiSendMessage("Application stepped over function to line " .. kiLine)
                kiSendMessage("DEBUG~~ECHO~~OFF DEBUG CONSOLE")
                kiSendMessage("DEBUG~~ECHO HIT BREAKPOINT")
                kiSendMessage(kiLine)
                kiSendMessage("DEBUG~~ECHO~~OFF HIT BREAKPOINT")
                kiDebugState = "kiStepIn"
                kiEvalVars = 1
                kiResumeDebug = 1
            end
        end
    end
    if kiEvalVars == 1 then
        if (kiMessageFormat == "xml") then
            tspdbg:sendstacktrace(2);
        end
        kiEvalVars = 0
    end
    if kiResumeDebug == 1 then
        kiDebug()
    end
end

local function ClearRemoteComm()
    if (not (ki.remotecomm) or not (ki.aborthook)) then
        error("This instrument's firmware does not support debugging. Please check the Keithley website for a firmware update for this model. In the meantime, you can debug by \"checking simulate\" in the Select Instrument dialog. Obviously, the simulator won't provide real data so your script might behave differently without modifications to simulate the data that you expect.");
    end
    local intercept = 0;
    intercept, ki.remotecomm.intercept = ki.remotecomm.intercept, intercept;
    local line = "not empty\n";
    while (line ~= nil) do
        line = ki.remotecomm.getmessage();
        logRemoteComm(line);
    end
    getmessageBuffer = {};
    intercept, ki.remotecomm.intercept = ki.remotecomm.intercept, intercept;
end

local function traceback(message)
    local trace = {};
    local level = 2;
    local info = debug.getinfo(level, "Snl");
    local _, _, errorsource, errorline, errormessage = string.find(message, '^%[string "([^"]*)"%]:(%d+): (.*)$');
    errorline = errorline ~= nil and tonumber(errorline);
    table.insert(trace, errorsource and errormessage or message);
    table.insert(trace, "\n");
    table.insert(trace, "stack trackback:\n");
    while (info ~= nil) do
        local _, _, short_src = string.find(info.short_src, '%[string "(.*%.%.%.)"%]');
        if (short_src) then
            if ((short_src == errorsource) and (info.currentline == errorline)) then
                errorsource = info.source or errorsource;
            end
            if (string.len(info.source) > 72) then
                short_src = string.sub(info.source, 1, 72) .. "...";
            else
                short_src = info.source;
            end
            short_src = string.format('[string %q]', short_src);
        else
            short_src = info.short_src;
        end
        table.insert(trace, "\t");
        table.insert(trace, string.format("%s:", short_src));
        if (info.currentline > 0) then
            table.insert(trace, string.format("%d:", info.currentline));
        end
        if (
            info.namewhat == "global" or info.namewhat == "local" or info.namewhat == "field" or
                info.namewhat == "method") then
            table.insert(trace, string.format(" in function `%s'", info.name));
        else
            if (info.what == "main") then
                table.insert(trace, " in main chunk");
            elseif (info.what == "C" or info.what == "tail") then
                table.insert(trace, " ?");
            else
                table.insert(trace, string.format(" in function <%s:%d>", short_src, info.linedefined));
            end
        end
        table.insert(trace, "\n");
        level = level + 1;
        info = debug.getinfo(level, "Snl");
    end
    if (errorsource) then
        trace[1] = string.format('[string "%s"]:%d:%s', errorsource, errorline, trace[1]);
    end
    return table.concat(trace);
end

---set value of a up variable in a stack frame and prints watchpoint and stack information
---@param stackLevel number stackLevel starts from zero level
---@param value any new value or expression to assign
---@param ... string heirarchy of table field. Example: To change value of "tab.x.y", variable arguments will be sent as "tab", "x", "y" in respective order
function kiSetUpVariable(stackLevel, value, ...)
    local retVal = nil
    if arg.n > 1 then
        retVal = tspdbg:kiSetStructureDataValue(stackLevel, value, arg)
    else
        retVal = tspdbg:setUpVariable(stackLevel, arg[1], value)
    end
    kiSendMessage(retVal, "setUpVariable")
    if (kiMessageFormat == "xml") then
        tspdbg:sendstacktrace(7);
    end
end

---set value of a local variable in a stack frame and prints watchpoint and stack information
---@param stackLevel number stackLevel starts from zero level
---@param value any new value or expression to assign
---@param ... string heirarchy of table field. Example: To change value of "tab.x.y", variable arguments will be sent as "tab", "x", "y" in respective order
function kiSetLocalVariable(stackLevel, value, ...)
    local retVal = nil
    if arg.n > 1 then
        retVal = tspdbg:kiSetStructureDataValue(stackLevel, value, arg)
    else
        retVal = tspdbg:setLocalVariable(stackLevel, arg[1], value)
    end
    kiSendMessage(retVal, "setLocalVariable")
    if (kiMessageFormat == "xml") then
        tspdbg:sendstacktrace(7);
    end
end

---set value of a global variable and prints watchpoint and stack information.
---@param stackLevel number stackLevel starts from zero level
---@param value string new value to assign
---@param ... string heirarchy of table field. Example: To change value of "tab.x.y", variable arguments will be sent as "tab", "x", "y" in respective order
function kiSetGlobalVariable(stackLevel, value, ...)
    local retVal = nil
    local success, value = tspdbg:evaluateExpression(value, stackLevel -1);
    if(success) then
        if arg.n == 1 then
            retVal = tspdbg:setGlobalVariable(arg[1], value)
        else
            varTable = _G[arg[1]]
            for i = 2, arg.n - 1, 1 do
                varTable = varTable[arg[i]]
            end
            varTable[arg[arg.n]] = value
            retVal= arg[arg.n]
        end
    else
        tspdbg:print("<SetVariable error ='" .. tspdbg:escape(tostring(value)) .. "' />");
    end
    kiSendMessage(retVal, "setGlobalVariable")
    if (kiMessageFormat == "xml") then
        tspdbg:sendstacktrace(7);
    end
end

function kiExecuteWithDebugger(source, sourceName, messageMode)
    kiMessageFormat = messageMode;
    kiSendMessage(nil, "session-begin", "kiDebuggerVersion", kiDebuggerVersion.version, "name", tostring(sourceName),
        "truncated-source", string.sub(source, 1, 128));
    local script, compilationError = loadstring(source, sourceName);
    local runOnExit = function() end;
    if (not (script)) then
        kiSendMessage(nil, "command-compilation-failed", "error", compilationError);
    else
        if (string.find(sourceName, "^TSB_Script")) then
            deleteScript(debug.getinfo(script), sourceName);
        end
        local function errorHandler(errorMessage)
            debug.sethook(nil);
            if (kiDebugState == "kiShellAbort") then
                return traceback(errorMessage);
            else
                return traceback(errorMessage);
            end
        end

        kiRunOnExit = function(f)
            local previous = runOnExit;
            runOnExit =
            function()
                previous();
                f();
            end;
        end;
        if (tspdbg) then
            tspdbg.usertopfunc = script;
            tspdbg.doscpi = doscpi;
            tspdbg.chunks = nil;
            collectgarbage();
        end
        kiDebuggedSource = source;
        kiDebuggedSourceName = sourceName;
        inDebugConsoleBlock = false;
        kiDebugState = "kiRun";
        local results = pack(xpcall(function() ClearRemoteComm(); debug.sethook(kiDebugHook, 'lcr'); script(); debug.sethook(); end
            , errorHandler));
        local runOk = table.remove(results, 1);
        if (not runOk) then
            kiSendMessage(nil, "execution-failed", "error", results[1], "name", tostring(sourceName));
        else
        end
    end
    kiSendMessage(nil, "session-end");
    runOnExit();
    if (tspdbg) then
        tspdbg.chunks = nil;
        tspdbg.excludeGlobal = nil
        collectgarbage();
    end
    kiMessageFormat = nil;
end
