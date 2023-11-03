local tspdbg = {};
_G['.tspdbg'] = tspdbg;
tspdbg.version = "2013.0519";
tspdbg.tableDictionary = {}
tspdbg.watchpointTable = {}
tspdbg.watchpoints = {}
tspdbg.expressionVariables = {}
tspdbg.excludeGlobal = {
    ["exit"] = true,
    ["fs"] = true,
    ["comm"] = true,
    ["table:"] = true,
    ["printbuffer"] = true,
    ["ACCESS_FULL"] = true,
    ["os"] = true,
    ["gettimewithfractional"] = true,
    ["ACCESS_EXCLUSIVE"] = true,
    ["getfenv"] = true,
    ["kiSetLocalVariable"] = true,
    ["pairs"] = true,
    ["_TRACEBACK"] = true,
    ["kiSetTableToStringFunction"] = true,
    ["assert"] = true,
    ["tonumber"] = true,
    ["io"] = true,
    ["eventlog"] = true,
    ["display"] = true,
    ["userstring"] = true,
    ["reset"] = true,
    ["node"] = true,
    ["kiSetWatchpoint"] = true,
    ["v"] = true,
    ["digio"] = true,
    ["_LOADED"] = true,
    ["_G"] = true,
    ["gpib"] = true,
    ["defbuffer1"] = true,
    [".tspdbg"] = true,
    ["available"] = true,
    ["makegetter"] = true,
    ["kiExecuteWithDebugger"] = true,
    ["coroutine"] = true,
    ["kiSetGlobalVariable"] = true,
    ["kiSetUpVariable"] = true,
    ["createconfigscript"] = true,
    ["_KIC"] = true,
    ["lan"] = true,
    ["dataqueue"] = true,
    ["kiClearWatchpoints"] = true,
    ["DISPLAY_RELEASE"] = true,
    ["kiClearBreakpoints"] = true,
    ["loadstring"] = true,
    ["getmetatable"] = true,
    ["scpi"] = true,
    ["kiDebuggerVersion"] = true,
    ["kic_kiDebugger"] = true,
    ["enum"] = true,
    ["delay"] = true,
    ["num_errs"] = true,
    ["accesslock"] = true,
    ["xpcall"] = true,
    ["kiClearWatchpoint"] = true,
    ["require"] = true,
    ["bit"] = true,
    ["_VERSION"] = true,
    ["format"] = true,
    ["tsplink"] = true,
    ["defbuffer2"] = true,
    ["errorqueue"] = true,
    ["table"] = true,
    ["status"] = true,
    ["DISPLAY_PROTECT"] = true,
    ["DISPLAY_LOCK"] = true,
    ["collectgarbage"] = true,
    ["trigger"] = true,
    ["buffer"] = true,
    ["setmetatable"] = true,
    ["next"] = true,
    ["tspnet"] = true,
    ["smu"] = true,
    ["ipairs"] = true,
    ["upgrade"] = true,
    ["memory"] = true,
    ["rawequal"] = true,
    ["printnumber"] = true,
    ["file"] = true,
    ["newproxy"] = true,
    ["define"] = true,
    ["settime"] = true,
    ["timer"] = true,
    ["ACCESS_LOCKOUT"] = true,
    ["info"] = true,
    ["rawset"] = true,
    ["ACCESS_PROTECTED"] = true,
    ["opc"] = true,
    ["string"] = true,
    ["tostring"] = true,
    ["gcinfo"] = true,
    ["waitcomplete"] = true,
    ["math"] = true,
    ["makesetter"] = true,
    ["pcall"] = true,
    ["unpack"] = true,
    ["kic_tspdbg"] = true,
    ["type"] = true,
    ["script"] = true,
    ["gettime"] = true,
    ["kiSetBreakpoint"] = true,
    ["dofile"] = true,
    ["beeper"] = true,
    ["rawget"] = true,
    ["ki"] = true,
    ["debug"] = true,
    ["__pow"] = true,
    ["setfenv"] = true,
    ["print"] = true,
    ["localnode"] = true,
    ["error"] = true,
    ["loadfile"] = true,
    ["getmessageBuffer"] = true,
    ["gm_vsweep"] = true,
    ["smub"] = true,
    ["serial"] = true,
    ["PulseVMeasureI"] = true,
    ["meminfo"] = true,
    ["ConfigPulseIMeasureVSweepLin"] = true,
    ["post_script"] = true,
    ["lptinit"] = true,
    ["lpt_init"] = true,
    ["General_test"] = true,
    ["preload_scp"] = true,
    ["savebuffer"] = true,
    ["run"] = true,
    ["i_leakage_threshold"] = true,
    ["i_leakage_measure"] = true,
    ["gm_isweep"] = true,
    ["PASSWORD_LAN"] = true,
    ["InitiatePulseTestDual"] = true,
    ["InitiatePulseTest"] = true,
    ["QueryPulseConfig"] = true,
    ["ConfigPulseVMeasureISweepLog"] = true,
    ["ConfigPulseVMeasureISweepLin"] = true,
    ["SweepIListMeasureV"] = true,
    ["ConfigPulseVMeasureI"] = true,
    ["ConfigPulseIMeasureV"] = true,
    ["PulseIMeasureV"] = true,
    ["SweepVListMeasureI"] = true,
    ["ConfigPulseIMeasureVSweepLog"] = true,
    ["PASSWORD_ALL"] = true,
    ["serialno"] = true,
    ["SweepILogMeasureV"] = true,
    ["SweepVLinMeasureI"] = true,
    ["SweepILinMeasureV"] = true,
    ["model"] = true,
    ["preloadfunctions"] = true,
    ["smua"] = true,
    ["revision"] = true,
    ["settimezone"] = true,
    ["SweepVLogMeasureI"] = true,
    ["PASSWORD_NONE"] = true,
    ["PASSWORD_WEB"] = true,
    ["setup"] = true,
    ["gettimezone"] = true,
    ["com_rst"] = true,
    ["MIDR"] = true,
    ["pulse_count"] = true,
    ["MAX_OFF"] = true,
    ["DFIFOSR"] = true,
    ["DAC_TIME"] = true,
    ["MAX_ON"] = true,
    ["get_adcs"] = true,
    ["pulser_measure"] = true,
    ["hex"] = true,
    ["TR"] = true,
    ["pulser_no_measure"] = true,
    ["FADCMCR"] = true,
    ["STORE"] = true,
    ["DEF_LEVELS"] = true,
    ["FVADCR"] = true,
    ["PLUS"] = true,
    ["pulse"] = true,
    ["FADCBPLLR"] = true,
    ["NZBRANCH"] = true,
    ["MAX_LEVEL"] = true,
    ["PDSCR_BIAS"] = true,
    ["PDSCR_OFF"] = true,
    ["fifo_rst"] = true,
    ["pregs"] = true,
    ["eint"] = true,
    ["MIN_OFF"] = true,
    ["MAX_DUTY_CYCLE"] = true,
    ["MBR"] = true,
    ["WAITZ"] = true,
    ["IRR"] = true,
    ["transfer"] = true,
    ["TUCK"] = true,
    ["FADCBPHLR"] = true,
    ["DP"] = true,
    ["rst"] = true,
    ["dint"] = true,
    ["INTERRUPT_MASK"] = true,
    ["NOT_INTERRUPT_MASK"] = true,
    ["_interrupts"] = true,
    ["ADP"] = true,
    ["CLSR"] = true,
    ["PORT48"] = true,
    ["PORT32"] = true,
    ["CreateSequence"] = true,
    ["calculate_dac_counts"] = true,
    ["SFADCR"] = true,
    ["AND"] = true,
    ["FADCMIR"] = true,
    ["RIGHT"] = true,
    ["SPR"] = true,
    ["DROP"] = true,
    ["LEFT"] = true,
    ["MOVE"] = true,
    ["OR"] = true,
    ["IPULSER"] = true,
    ["INV"] = true,
    ["waitz"] = true,
    ["NEXTP"] = true,
    ["RFROM"] = true,
    ["TOR"] = true,
    ["pulse_w_measure"] = true,
    ["IDCR"] = true,
    ["ADCTR"] = true,
    ["HOLD"] = true,
    ["GREZBRANCH"] = true,
    ["SWAP"] = true,
    ["FADCAPHLR"] = true,
    ["EEPROMDR"] = true,
    ["DUP"] = true,
    ["MINUS"] = true,
    ["NUP"] = true,
    ["DEF_PERIOD"] = true,
    ["pulser_oo"] = true,
    ["LITSTORE"] = true,
    ["LITAT"] = true,
    ["pulser_with_measure"] = true,
    ["DEF_WIDTH"] = true,
    ["IBIASR"] = true,
    ["ANDOR"] = true,
    ["EXOR"] = true,
    ["ZBRANCH"] = true,
    ["RTS"] = true,
    ["PORT16"] = true,
    ["SequenceToolsVersion"] = true,
    ["LIT"] = true,
    ["RAT"] = true,
    ["WAITNZ"] = true,
    ["FADCBNLLR"] = true,
    ["InstallSeqTools"] = true,
    ["FADCBNHLR"] = true,
    ["TRR"] = true,
    ["PDSCR_PULSE"] = true,
    ["move"] = true,
    ["FADCANLLR"] = true,
    ["EEPROMCR"] = true,
    ["PDSCR"] = true,
    ["FADCAPLLR"] = true,
    ["SFADCLR"] = true,
    ["MIN_LEVEL"] = true,
    ["FIADCR"] = true,
    ["OCR"] = true,
    ["GTR"] = true,
    ["EMR"] = true,
    ["EDR"] = true,
    ["readback"] = true,
    ["ECR"] = true,
    ["pulser_sweep"] = true,
    ["TR2"] = true,
    ["DCR"] = true,
    ["EEPROMSR"] = true,
    ["ADD_TO"] = true,
    ["RIR"] = true,
    ["TSR"] = true,
    ["bias_pulse_dac"] = true,
    ["TCR"] = true,
    ["FADCFCR"] = true,
    ["ADCCR"] = true,
    ["FADCANHLR"] = true,
    ["jsr"] = true,
    ["NVLR"] = true,
    ["JSR"] = true,
    ["prog"] = true,
    ["pf"] = true,
    ["branch"] = true,
    ["VRR"] = true,
    ["NIP"] = true,
    ["calibrate_meas"] = true,
    ["pulse_offset"] = true,
    ["DPRT"] = true,
    ["get_cal_constants"] = true,
    ["DEF_OFF"] = true,
    ["TRANSFER"] = true,
    ["ODD_WAIT"] = true,
    ["LSRL"] = true,
    ["waitforfifo"] = true,
    ["MIN_ON"] = true,
    ["PVLR"] = true,
    ["NEG"] = true,
    ["process"] = true,
    ["EER"] = true,
    ["OVER"] = true,
    ["EEPROMAR"] = true,
    ["PASR"] = true,
    ["FETCH"] = true,
    ["BRANCH"] = true,
    ["isExtDispAPI"] = true,
    ["getSwitchModule"] = true,
    ["TSP_Common_Functions"] = true,
    ["tc"] = true,
    ["extInOutSupported"] = true,
    ["http"] = true,
    ["copy_table"] = true,
    ["getCommonVersion"] = true,
    ["negate_array"] = true,
    ["scan"] = true,
    ["check_display_tc"] = true,
    ["removeDisplayErrors"] = true,
    ["isWithinPercent"] = true,
    ["getMeasureModule"] = true,
    ["tsplinkSupported"] = true,
    ["printLocalnodeVersion"] = true,
    ["randomizeScpiCommand"] = true,
    ["getIPAddress"] = true,
    ["dmm"] = true,
    ["PlatformTSP"] = true,
    ["SetupUnit1"] = true,
    ["timerStimulus"] = true,
    ["getTimer"] = true,
    ["channel"] = true,
    ["buildScpiCommand"] = true,
    ["modulo"] = true,
    ["autoexec"] = true,
    ["syncTime"] = true,
    ["printTestCaseSummary"] = true,
    ["removeErrors"] = true,
    ["slot"] = true,
    ["check_pixel_tc"] = true,
    ["fan"] = true,
    ["isWithinRange"] = true,
    ["localnodeVersion"] = true,
    ["printScriptHeader"] = true,
    ["cal"] = true,
    ["digioSupported"] = true,
    ["usbDriveExists"] = true,
    ["Setup00"] = true,
    ["findTSPNetInstr"] = true,
    ["timerOverrun"] = true,
    ["get_family"] = true,
    ["printToDisplay"] = true,
    ["parseRCS"] = true,
    ["isPlatformModel"] = true,
    ["sendScpiCommands"] = true,
    ["[1]=table:"] = true,
    ["measurelist1"] = true,
    ["getErrorQueueCount"] = true,
    ["check_status_tc"] = true,
    ["findTSPLinkInstr"] = true,
    ["clearTimer"] = true,
    ["check_digio_plug"] = true,
}

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

deleteScript(debug.getinfo(1), "tspdbg");
tspdbg.valueLengthLimit = 1024;
tspdbg.tag = 'tspdbg' .. '-AA4E9540-A46C-4671-81D7-4FE69A9B6DC4';
tspdbg.startTag = "<" .. tspdbg.tag .. ">";
tspdbg.endTag = "</" .. tspdbg.tag .. ">";
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
function tspdbg:log(...)
    if (openlogger) then
        local logger = openlogger();
        logger:write(string.format("%s\t", os.date("%Y-%m-%d %H:%M:%S")));
        table.foreachi(arg, function(_, v) logger:write(tostring(v) .. "\t"); end);
        logger:write("\n");
        logger:close();
    end
end

function tspdbg:dataToJson(o, currentLevel, endLevel)
	local s = ''
	if type(o) == "table" then
        if currentLevel == endLevel then
            return '"expansion not allowed"'
        end
		s = s .. '{ "table" : ['
		local obj_str = ''
        local i = 0
		for k, v in pairs(o) do
			if string.len(obj_str) > 0 then
				obj_str = obj_str .. ','
			end
			local rhs = tspdbg:dataToJson(v, currentLevel + 1, endLevel)
            obj_str = obj_str .. '{ "name":'..tspdbg:dataToJson(k, currentLevel + 1, endLevel).. ', "value" :' ..rhs ..'}'
		end
		s = s .. obj_str .. ']}'
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
	elseif type(o) == "function" then
		s = s .. '"FUNCTION"'
	elseif type(o) == "'userdata'" then
		s = s .. '"USERDATA"'
	elseif type(o) == "thread" then
		s = s .. '"THREAD"'
	else
		s = s .. '' .. type(o) .. ''
	end

	return s
end

---it returns length of a table
---@param tab table parameter
---@return number len length of table
function tspdbg:getTableLength(tab)
    local len = 0
    if tab == nil then return 0 end
    for _, _ in pairs(tab) do
        len = len + 1
    end
    return len
end

local printBuffer = {};
function tspdbg:print(...)
    self:log(unpack(arg));
    local n = table.getn(arg);
    for i = 1, n do
        table.insert(printBuffer, arg[i]);
    end
    if (arg[n] == self.endTag) then
        self:printflush();
    end
end

function tspdbg:printflush()
    print(table.concat(printBuffer, "\n"));
    printBuffer = {};
end

function tspdbg:pack(...)
    return arg;
end

function tspdbg:escape(s)
    local byteStrings = {};
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

function tspdbg.tabletostring(t)
    local s = {};
    local function formatvalue(v)
        local vtype = type(v);
        if (vtype == "number") then
            return tostring(v);
        elseif (vtype == "string") then
            return string.format("%q", tspdbg.tolimitedstring(tspdbg, v));
        end
        return tspdbg.tolimitedstring(tspdbg, v);
    end

    local function isName(s)
        return (string.find(s, "^[_%a][_%a%d]*$") ~= nil);
    end

    table.insert(s, "{ ");
    local sep = "";
    local n = table.getn(t);
    local ellipsisstart = n + 1;
    local ellipsisend = n + 1;
    if (n > 15) then
        ellipsisstart = 6;
        ellipsisend = n - 10;
    end
    local numerickeys = {};
    local stringkeys = {};
    for key, _ in pairs(t) do
        if (type(key) == "number") then
            table.insert(numerickeys, key);
        elseif (type(key) == "string") then
            table.insert(stringkeys, key);
        else
        end
    end
    table.sort(numerickeys);
    table.sort(stringkeys);
    for _, key in ipairs(numerickeys) do
        local i = key;
        local v = t[i];
        if ((i < ellipsisstart) or (i > ellipsisend)) then
            table.insert(s, sep);
            sep = ", ";
            table.insert(s, "[");
            table.insert(s, tostring(i));
            table.insert(s, "]=");
            table.insert(s, formatvalue(v));
        elseif (i == ellipsisstart) then
            table.insert(s, sep);
            sep = ", ";
            table.insert(s, "...");
        end
    end
    for _, key in pairs(stringkeys) do
        local k = key;
        local v = t[k];
        table.insert(s, sep);
        sep = ", ";
        if (isName(k)) then
            table.insert(s, k);
        else
            table.insert(s, "[");
            table.insert(s, string.format("%q", k));
            table.insert(s, "]");
        end
        table.insert(s, "=");
        table.insert(s, formatvalue(v));
    end
    table.insert(s, " }");
    return table.concat(s);
end

function tspdbg:tolimitedstring(value)
    local s = tostring(value);
    local len = string.len(s);
    if (len > self.valueLengthLimit) then
        s = string.format("%s (bytes %i to %i omitted)", string.sub(s, 1, self.valueLengthLimit),
            self.valueLengthLimit + 1, len);
    end
    return s;
end

function tspdbg:sendstacktrace(topSkipCount)
    self:print(self.startTag);
    self:print("  <stacks>");
    topSkipCount = topSkipCount or 6;
    local bottomSkipCount = 4;
    local traceLines = {};
    for frame in string.gfind(debug.traceback(), "[ \t]*([^\n]+)") do
        table.insert(traceLines, frame);
    end
    for i = 1, topSkipCount + 1 do
        table.remove(traceLines, 1);
    end
    local userPlusBottomCount = table.getn(traceLines);
    local userCount = userPlusBottomCount - bottomSkipCount;
    for i = userPlusBottomCount, userCount + 1, -1 do
        table.remove(traceLines, i);
    end
    for level = 0, userCount - 1 do
        local funcinfo = debug.getinfo(level + topSkipCount + 1);
        local currentline = self:escape(tostring(funcinfo.currentline));
        local func = self:escape(tostring(funcinfo.func));
        local linedefined = self:escape(tostring(funcinfo.linedefined));
        local name = self:escape(funcinfo.name or ("(defined at line " .. tostring(linedefined) .. ")"));
        local namewhat = self:escape(tostring(funcinfo.namewhat));
        local nups = self:escape(tostring(funcinfo.nups));
        local short_src = self:escape(tostring(funcinfo.short_src));
        local source = self:escape(string.sub(tostring(funcinfo.source), 1, 256));
        local what = self:escape(tostring(funcinfo.what));
        self:print("    <stack level='" .. level ..
            "' currentline='" .. currentline ..
            "' func='" .. func ..
            "' linedefined='" .. linedefined ..
            "' name='" .. name ..
            "' namewhat='" .. namewhat ..
            "' nups='" .. nups ..
            "' short_src='" .. short_src ..
            "' source='" .. source ..
            "' what='" .. what ..
            "' frame='" .. self:escape(traceLines[level + 1]) ..
            "'>");
        self:sendWatchpoints(level - 3 + topSkipCount);
        self:sendglobals(level - 3 + topSkipCount, true, true);
        self:sendupvalues(level - 3 + topSkipCount, true);
        self:sendlocals(level - 3 + topSkipCount, true);
        self:print("    </stack>");
    end
    self:print("  </stacks>");
    self:print(self.endTag);
end

function tspdbg:sendlocals(level, nested)
    if (not (nested)) then
        self:print(self.startTag);
        self:print("  <locals level='" .. level .. "'>");
    else
        self:print("  <locals>");
    end
    for i = 1, 99 do
        local success, name, value = pcall(debug.getlocal, level + 6, i);
        if success then
            if name then
                local tableData = ""
                if string.starts(type(value), 'table') then
                    tspdbg.tableDictionary[name] = value
                    tableData = "' tableData='"..tostring(tspdbg:dataToJson(value, 0, 10))
                end
                self:print("    <local name='" ..
                    self:escape(name) ..
                    "' value='" .. self:escape(self:tolimitedstring(value)) .. "' type='" .. type(value) ..tableData.."' />");
            else
                break;
            end
        else
            self:print("    <error message='" .. self:escape(name) .. "' />");
            break;
        end
    end
    self:print("  </locals>");
    if (not (nested)) then
        self:print(self.endTag);
    end
end

function tspdbg:sendupvalues(level, nested)
    if (not (nested)) then
        self:print(self.startTag);
        self:print("  <upvalues level='" .. level .. "'>");
    else
        self:print("  <upvalues>");
    end
    for i = 1, 99 do
        local success, name, value = pcall(function() return debug.getupvalue(debug.getinfo(level + 7).func, i); end, i);
        if success then
            if name then
                local tableData = ""
                if string.starts(type(value), 'table') then
                    tspdbg.tableDictionary[name] = value
                    tableData = "' tableData='"..tostring(tspdbg:dataToJson(value, 0, 10))
                end
                self:print("    <upvalue name='" ..
                    self:escape(name) ..
                    "' value='" .. self:escape(self:tolimitedstring(value)) .. "' type='" .. type(value) ..tableData.. "' />");
            else
                break;
            end
        else
            self:print("    <error message='" .. self:escape(name) .. "' />");
            break;
        end
    end
    self:print("  </upvalues>");
    if (not (nested)) then
        self:print(self.endTag);
    end
end

---printWatchpoints print watchpoints xml
---@param watchpointTable table<string, any>
function tspdbg:printWatchpoints(watchpointTable)
    self:print("  <watchpoints>");
    for key,value in pairs(watchpointTable) do
        self:print("    <watchpoint expression='" ..
                    self:escape(tostring(key)) ..
                    "' value='" .. self:escape(self:tolimitedstring(value)) .. "' type='" .. type(value) .. "' />");
    end

    self:print("</watchpoints>")
end

---Updates expressions with values, that are present in scope, in watchpointTable.
---@param stackLevel number stack frame level starts from zero
function tspdbg:sendWatchpoints(stackLevel)
        local watchpointTable = {}
        local len = tspdbg:getTableLength(tspdbg.watchpoints)
        if(len > 0) then
            local env = tspdbg:createExpressionEnvironment(tspdbg.expressionVariables, stackLevel + 5);
            for watchLoop = 1, len do
                local watchpoint = tspdbg.watchpoints[watchLoop]
                if watchpoint.enabled then
                    local success, value = watchpoint:evaluateInEnvironment(env);
                    if (success) then
                        watchpointTable[watchpoint.source] = value
                    else
                        if (value ~= nil) then
                            -- invalid watch expression and it's value added
                            watchpointTable[watchpoint.source] = value
                        end
                    end
                end
            end
        end
        tspdbg:printWatchpoints(watchpointTable)
    end

function tspdbg:sendglobals(level, nested, omitfunction)
    if (not (nested)) then
        self:print(self.startTag);
        self:print("  <globals level='" .. level .. "'>");
    else
        self:print("  <globals>");
    end
    local success, info = pcall(function() return debug.getinfo(level + 7); end);
    if success then
        function printGlobals() 
            if (not (omitfunction)) then
                self:print("    <function name='" ..
                self:escape(tostring(info.name)) ..
                "' source='" ..
                self:escape(tostring(info.source)) ..
                "' linedefined='" .. self:escape(info.linedefined or -1) .. "' />");
            end
            local envVar = getfenv(info.func)

            for name, val in pairs(envVar) do
                if(tspdbg.excludeGlobal[name] == nil and name ~= nil and string.starts(tostring(name), 'table:') == false) then
                    local value = _G[name]
                    local tableData = ""
                    local metaTable = getmetatable(_G[name])
                    local isTable = string.starts(type(value), 'table')
                    local showTable = isTable and (metaTable == nil or ((metaTable["Getters"] == nil or metaTable["Setters"] == nil or metaTable["Objects"] == nil)))
                    
                    if showTable then
                        tableData = "' tableData='"..tostring(tspdbg:dataToJson(value, 0, 10))
                    end
                    if isTable == false or showTable then
                    self:print("    <global name='" ..
                        self:escape(tostring(name)) ..
                        "' value='" .. self:escape(self:tolimitedstring(value)) .. "' type='" .. type(value) ..tableData.."' />");
                    end
                end
            end
        end
        local success = pcall(printGlobals)
        if success == false then
            self:print("    <error message='" .. self:escape("error in sending globals") .. "' />");
        end
    end
    self:print("  </globals>");
    if (not (nested)) then
        self:print(self.endTag);
    end
end

function tspdbg:dostring(source, id, sendStackTrace)
    local executed;
    self:print(self.startTag);
    if (id) then self:print("<" .. id .. ">"); end
    if (string.sub(source, 1, 1) == "*") then
        executed = false;
        self:print("  <dostring scpi='" .. self:escape(source) .. "'>");
        local output = self.doscpi(source);
        if (output) then
            self:print(output);
        end
    else
        self:print("  <dostring source='" .. self:escape(source) .. "'>");
        local script, compilationError = loadstring(source);
        if (not script) then
            executed = false;
            self:print("    <compilation-failed error='" .. self:escape(compilationError) .. "' />");
        else
            executed = true;
            local results = self:pack(xpcall(script, debug.traceback));
            local runOk = table.remove(results, 1);
            if (not runOk) then
                local _, emptytrace = xpcall(error, debug.traceback);
                local discardlen = string.len(emptytrace) - string.find(emptytrace, "\n[^\n]*xpcall");
                local message = string.sub(results[1], 1, string.len(results[1]) - discardlen);
                self:print("    <execution-failed error='" .. self:escape(message) .. "' />");
            else
                table.foreachi(results, function(_, result)
                    self:print("    <result value='" ..
                        self:escape(self:tolimitedstring(result)) .. "' type='" .. type(result) .. "' />");
                end);
            end
        end
    end
    self:print("  </dostring>");
    if (id) then self:print("</" .. id .. ">"); end
    self:print(self.endTag);
    if (executed and sendStackTrace) then
        self:sendstacktrace(1 + 6);
    end
end

---evaluate a string expression in specified stack level
---@param expression string expression to be evaluated
---@param stackLevel number stack level
---@return boolean success returns true if expression is valid otherwise it returns false
---@return string|number|table|nil value returns the value of expression
function tspdbg:evaluateExpression(expression, stackLevel)
    local retVal = tspdbg:compileExpression(expression)
    if(retVal.compilationError) then
        return false, nil
    end
    local variables = {}
    for key in pairs(retVal.variables) do
        variables[key] = true;
    end
    local env = tspdbg:createExpressionEnvironment(variables, stackLevel + 9);
    local success, value = retVal:evaluateInEnvironment(env);
    return success, value
end

--- it sets value of index/name key of table.
---@param stackLevel number starts from zero
---@param expressionValue string value or expression to set e.g: "nil"
---@param argTable table variable arguments e.g: "tab", "x", "y"
---@return string | nil retVal if success it returns name of variable otherwise it returns nil
function tspdbg:kiSetStructureDataValue(stackLevel, expressionValue, argTable)

    local varTable = tspdbg.tableDictionary[argTable[1]]
    for i = 2, argTable.n - 1, 1 do
        varTable = varTable[argTable[i]]
    end
    local success, value = tspdbg:evaluateExpression(expressionValue, stackLevel);
    if success then
        varTable[argTable[argTable.n]] = value
        return argTable[argTable.n]
    else
        self:print("<SetVariable error ='" .. self:escape(value) .. "' />");
    end
    return nil
end

local function getFunctionVariables(func)
    local globals = {};
    if func then
        local dumpsuccess, bytes = pcall(string.dump, func);
        if dumpsuccess then
            local chunk = _G['.tspdbg']:ChunkSpy(nil, bytes);
            table.foreach(chunk.func.k, function(BxPlus1, name)
                for pc, inst in chunk.func.inst do
                    if (inst.Bx == BxPlus1 - 1) then
                        if (inst.opname == "setglobal") or (inst.opname == "getglobal") then
                            globals[name] = true;
                        end
                    end
                end
            end);
        else
        end
    end
    return globals;
end

function tspdbg:createExpressionEnvironment(variables, level)
    local env = {};
    local funcinfo = debug.getinfo(level + 1, "fS");
    local fenv = getfenv(funcinfo.func);
    for name in pairs(variables) do
        local found;
        if (funcinfo and (funcinfo.what ~= "tail")) then
            local index = 1;
            local localname, value = debug.getlocal(level + 1, index);
            while (localname) do
                if (localname == name) then
                    found = { location = "local", value = value };
                end
                index = index + 1;
                localname, value = debug.getlocal(level + 1, index);
            end
            if (not (found)) then
                index = 1;
                local upvaluename, value = debug.getupvalue(funcinfo.func, index);
                while (upvaluename) do
                    if (upvaluename == name) then
                        found = { location = "up-value", value = value };
                        break;
                    end
                    index = index + 1;
                    upvaluename, value = debug.getupvalue(funcinfo.func, index);
                end
            end
        end
        if (found) then
            env[name] = found.value;
        else
            env[name] = fenv[name];
        end
    end
    return env;
end

function tspdbg:compileExpression(source)
    local expression = {};
    expression.source = source;
    expression.func, expression.compilationError = loadstring("return " .. source .. "\n;");
    expression.variables = getFunctionVariables(expression.func);
    function expression:evaluate(startingLevel)
        return self:evaluateInEnvironment(self:createExpressionEnvironment(self.variables, startingLevel));
    end

    function expression:evaluateInEnvironment(env)
        if (self.compilationError) then
            return false;
        end
        setfenv(self.func, env);
        return pcall(self.func);
    end

    return expression;
end

function tspdbg:eval(expression, level)
    if (not (expression.func)) then
        return false, "compilation-failed", expression.compilationError;
    end
    local success, result = expression:evaluate(level);
    if (success) then
        return true, result;
    else
        return false, "execution-failed", result;
    end
end

---set value of a up variable in a stack frame
---@param stackLevel number stackLevel starts from zero level
---@param varName string name of the variable
---@param value any new value to assign
---@return string|nil retVal returns name of variable if variable found otherwise returns nil
function tspdbg:setUpVariable(stackLevel, varName, value)
    local kiStackLevel = stackLevel + 8
    local retVal = nil
    local dbugInfo = debug.getinfo(kiStackLevel, "f")
    if (dbugInfo) then
        local i = 1
        while (true) do
            local var = debug.getupvalue(dbugInfo.func, i)
            if not (var) then break end
            if var == varName then
                local success, setVal = tspdbg:evaluateExpression(value, stackLevel)
                if success then
                    retVal = debug.setupvalue(dbugInfo.func, i, setVal)
                else
                    self:print("<SetVariable error ='" .. self:escape(tostring(setVal)) .. "' />");
                    return nil
                end
            end
            i = i + 1
        end
    end
    return retVal
end

---set value of a local variable in a stack frame
---@param stackLevel number stackLevel starts from zero level
---@param varName string name of the variable
---@param value any new value to assign
---@return string|nil retVal returns name of variable if variable found otherwise returns nil
function tspdbg:setLocalVariable(stackLevel, varName, value)
    local kiStackLevel = stackLevel + 8
    local retVal = nil
    local dbugInfo = debug.getinfo(kiStackLevel)
    if (dbugInfo) then
        local i = 1
        while (true) do
            local var = debug.getlocal(kiStackLevel, i)
            if not (var) then break end
            if var == varName then
                local success, setVal = tspdbg:evaluateExpression(value, stackLevel)
                if success then
                    retVal = debug.setlocal(kiStackLevel, i, setVal)
                else
                    self:print("<SetVariable error ='" .. self:escape(tostring(setVal)) .. "' />");
                    return nil
                end
            end
            i = i + 1
        end
    end
    return retVal
end

---set value of a global variable
---@param varName string name of the variable
---@param value any new value to assign
---@return string|nil retVal returns name of variable if variable found otherwise returns nil
function tspdbg:setGlobalVariable(varName, value)
    if (_G[varName]) then
        _G[varName] = value
        return varName
    end
    return nil
end

function tspdbg:evaluate(source, id)
    local expression = self:compileExpression(source);
    self:print(self.startTag);
    if (id) then self:print("<" .. id .. ">"); end
    self:print("  <evaluate expression='" .. self:escape(source) .. "'>");
    local externallevels = 4;
    local success, result, message = self:eval(expression, externallevels + 2);
    if (success) then
        self:print("    <result type='" ..
            self:escape(type(result)) .. "' value='" .. self:escape(self:tolimitedstring(result)) .. "' />");
    else
        self:print("    <" .. result .. " error='" .. self:escape(message) .. "' />");
    end
    self:print("  </evaluate>");
    if (id) then self:print("</" .. id .. ">"); end
    self:print(self.endTag);
end

function tspdbg:getGlobals(f)
    local globals = {};
    local chunkinfo = self:ChunkSpy(tostring(f), string.dump(f));
    table.foreach(chunkinfo.func.inst, function(_, inst)
        if (inst.opname == "setglobal") then
            local name = chunkinfo.func.k[inst.Bx + 1];
            globals[name] = true;
        elseif (inst.opname == "getglobal") then
            local name = chunkinfo.func.k[inst.Bx + 1];
            globals[name] = true;
        end
    end);
    local list = {};
    table.foreach(globals, function(k, _) table.insert(list, k); end);
    return list;
end

function tspdbg:getLineNumbers(f)
    local lines = {};
    local chunkinfo = self:ChunkSpy(tostring(f), string.dump(f));
    local function doFunc(func)
        if (not (func)) then error("func cannot be nil"); end
        if (not (func.lineinfo)) then error("func.lineinfo cannot be nil"); end
        if (not (func.p)) then error("func.p cannot be nil"); end
        local function doLineInfo(f)
            if (not (f)) then error("f cannot be nil"); end
            if (not (f.lineinfo)) then error("f.lineinfo cannot be nil"); end
            table.foreachi(f.lineinfo, function(_, line)
                lines[line] = line;
            end);
        end

        local function doP(p)
            if (not (p)) then error("p cannot be nil"); end
            doLineInfo(p);
            if (p.p) then
                table.foreach(p.p, function(_, v) doP(v); end);
            end
        end

        doLineInfo(func);
        table.foreach(func.p, function(_, v) doP(v); end);
    end

    doFunc(chunkinfo.func);
    local numbers = {};
    table.foreach(lines, function(_, v) table.insert(numbers, v); end);
    table.sort(numbers);
    return numbers, chunkinfo.func.lineDefined;
end

function tspdbg:ChunkSpy(chunk_name, chunk)
    local CONFIGURATION = {
        ["x86 standard"] = {
            description = "x86 standard (32-bit, little endian, doubles)",
            endianness = 1,
            size_int = 4,
            size_size_t = 4,
            size_Instruction = 4,
            size_lua_Number = 8,
            SIZE_OP = 6,
            SIZE_A = 8,
            SIZE_B = 9,
            SIZE_C = 9,
            number_type = "double",
            MAX_STACK = 250,
        },
    }
    local config = {}
    local function SetProfile(profile)
        if profile == "local" then
            local flag1, flag2 = config.DISPLAY_FLAG, config.AUTO_DETECT
            config.DISPLAY_FLAG, config.AUTO_DETECT = false, true
            local LUA_SAMPLE = string.dump(function() end)
            local ok, _ = pcall(tspdbg.ChunkSpy, "", LUA_SAMPLE)
            if not ok then error("error compiling sample to test local profile") end
            config.DISPLAY_FLAG, config.AUTO_DETECT = flag1, flag2
        else
            local c = CONFIGURATION[profile]
            if not c then return false end
            for i, v in pairs(c) do config[i] = v end
        end
        return true
    end

    SetProfile("x86 standard")
    config.SIGNATURE = "\27Lua"
    config.TEST_NUMBER = 3.14159265358979323846E7
    config.LUA_TNIL = 0
    config.LUA_TNUMBER = 3
    config.LUA_TSTRING = 4
    config.VERSION = 80
    config.FPF = 32
    config.DISPLAY_FLAG = true
    config.DISPLAY_BRIEF = nil
    config.DISPLAY_INDENT = nil
    config.STATS = nil
    config.DISPLAY_OFFSET_HEX = true
    config.DISPLAY_SEP = "  "
    config.DISPLAY_COMMENT = "; "
    config.DISPLAY_HEX_DATA = true
    config.WIDTH_HEX = 8
    config.WIDTH_OFFSET = nil
    config.DISPLAY_LOWERCASE = true
    config.WIDTH_OPCODE = nil
    config.VERBOSE_TEST = false
    local convert_from = {}
    local convert_to = {}
    local function grab_byte(v)
        return math.floor(v / 256), string.char(math.mod(math.floor(v), 256))
    end

    local TEST_NUMBERS = {
        ["double"] = "\182\9\147\104\231\245\125\65",
        ["single"] = "\59\175\239\75",
        ["int"] = "\118\94\223\1",
        ["long long"] = "\118\94\223\1\0\0\0\0",
    }
    convert_from["double"] = function(x)
        local sign = 1
        local mantissa = math.mod(string.byte(x, 7), 16)
        for i = 6, 1, -1 do mantissa = mantissa * 256 + string.byte(x, i) end
        if string.byte(x, 8) > 127 then sign = -1 end
        local exponent = math.mod(string.byte(x, 8), 128) * 16 +
            math.floor(string.byte(x, 7) / 16)
        if exponent == 0 then return 0 end
        mantissa = (math.ldexp(mantissa, -52) + 1) * sign
        return math.ldexp(mantissa, exponent - 1023)
    end
    convert_from["single"] = function(x)
        local sign = 1
        local mantissa = math.mod(string.byte(x, 3), 128)
        for i = 2, 1, -1 do mantissa = mantissa * 256 + string.byte(x, i) end
        if string.byte(x, 4) > 127 then sign = -1 end
        local exponent = math.mod(string.byte(x, 4), 128) * 2 +
            math.floor(string.byte(x, 3) / 128)
        if exponent == 0 then return 0 end
        mantissa = (math.ldexp(mantissa, -23) + 1) * sign
        return math.ldexp(mantissa, exponent - 127)
    end
    convert_from["int"] = function(x)
        local sum = 0
        for i = config.size_lua_Number, 1, -1 do
            sum = sum * 256 + string.byte(x, i)
        end
        if string.byte(x, config.size_lua_Number) > 127 then
            sum = sum - math.ldexp(1, 8 * config.size_lua_Number)
        end
        return sum
    end
    convert_from["long long"] = convert_from["int"]
    convert_to["double"] = function(x)
        local sign = 0
        if x < 0 then sign = 1;
        x = -x
        end
        local mantissa, exponent = math.frexp(x)
        if x == 0 then
            mantissa, exponent = 0, 0
        else
            mantissa = (mantissa * 2 - 1) * math.ldexp(0.5, 53)
            exponent = exponent + 1022
        end
        local v, byte = ""
        x = mantissa
        for i = 1, 6 do
            x, byte = grab_byte(x);
            v = v .. byte
        end
        x, byte = grab_byte(exponent * 16 + x);
        v = v .. byte
        x, byte = grab_byte(sign * 128 + x);
        v = v .. byte
        return v
    end
    convert_to["single"] = function(x)
        local sign = 0
        if x < 0 then sign = 1;
        x = -x
        end
        local mantissa, exponent = math.frexp(x)
        if x == 0 then
            mantissa = 0;
            exponent = 0
        else
            mantissa = (mantissa * 2 - 1) * math.ldexp(0.5, 24)
            exponent = exponent + 126
        end
        local v, byte = ""
        x, byte = grab_byte(mantissa);
        v = v .. byte
        x, byte = grab_byte(x);
        v = v .. byte
        x, byte = grab_byte(exponent * 128 + x);
        v = v .. byte
        x, byte = grab_byte(sign * 128 + x);
        v = v .. byte
        return v
    end
    convert_to["int"] = function(x)
        local v = ""
        x = math.floor(x)
        if x >= 0 then
            for i = 1, config.size_lua_Number do
                v = v .. string.char(math.mod(x, 256));
                x = math.floor(x / 256)
            end
        else
            x = -x
            local carry = 1
            for i = 1, config.size_lua_Number do
                local c = 255 - math.mod(x, 256) + carry
                if c == 256 then c = 0;
                carry = 1
                else carry = 0 end
                v = v .. string.char(c);
                x = math.floor(x / 256)
            end
        end
        return v
    end
    convert_to["long long"] = convert_to["int"]
    local function WidthOf(n) return string.len(tostring(n)) end

    local function DecodeInit()
        config.SIZE_Bx = config.SIZE_B + config.SIZE_C
        local MASK_OP = math.ldexp(1, config.SIZE_OP)
        local MASK_B = math.ldexp(1, config.SIZE_B)
        local MASK_C = math.ldexp(1, config.SIZE_C)
        local MASK_Bx = math.ldexp(1, config.SIZE_Bx)
        local MASK_A = math.ldexp(1, config.SIZE_A)
        config.MAXARG_sBx = math.floor((MASK_Bx - 1) / 2)
        config.iABC = {
            config.SIZE_OP,
            config.SIZE_C,
            config.SIZE_B,
            config.SIZE_A,
        }
        config.mABC = { MASK_OP, MASK_C, MASK_B, MASK_A, }
        config.nABC = { "OP", "C", "B", "A", }
        local op =
        [[
	MOVE LOADK LOADBOOL LOADNIL GETUPVAL
    GETGLOBAL GETTABLE SETGLOBAL SETUPVAL SETTABLE
    NEWTABLE SELF ADD SUB MUL
    DIV POW UNM NOT CONCAT
    JMP EQ LT LE TEST
    CALL TAILCALL RETURN FORLOOP TFORLOOP
    TFORPREP SETLIST SETLISTO CLOSE CLOSURE
]]
        config.opnames = {}
        config.NUM_OPCODES = 0
        if not config.WIDTH_OPCODE then config.WIDTH_OPCODE = 0 end
        for v in string.gfind(op, "[^%s]+") do
            if config.DISPLAY_LOWERCASE then
                config.opnames[config.NUM_OPCODES] = string.lower(v)
            else
                config.opnames[config.NUM_OPCODES] = v
            end
            local vlen = string.len(v)
            if vlen > config.WIDTH_OPCODE then
                config.WIDTH_OPCODE = vlen
            end
            config.NUM_OPCODES = config.NUM_OPCODES + 1
        end
        config.opmode = "01000101000000000000200000002021101"
        config.WIDTH_A = WidthOf(MASK_A)
        config.WIDTH_B = WidthOf(MASK_B)
        config.WIDTH_C = WidthOf(MASK_C)
        config.WIDTH_Bx = WidthOf(MASK_Bx) + 1
        config.FORMAT_A = string.format("%%-%dd", config.WIDTH_A)
        config.FORMAT_B = string.format("%%-%dd", config.WIDTH_B)
        config.FORMAT_C = string.format("%%-%dd", config.WIDTH_C)
        config.PAD_Bx = config.WIDTH_A + config.WIDTH_B + config.WIDTH_C + 2
            - config.WIDTH_Bx
        if config.PAD_Bx > 0 then
            config.PAD_Bx = string.rep(" ", config.PAD_Bx)
        else
            config.PAD_Bx = ""
        end
        config.FORMAT_Bx = string.format("%%-%dd", config.WIDTH_Bx)
        config.FORMAT_AB = string.format("%s %s %s", config.FORMAT_A, config.FORMAT_B, string.rep(" ", config.WIDTH_C))
        config.FORMAT_ABC = string.format("%s %s %s", config.FORMAT_A, config.FORMAT_B, config.FORMAT_C)
        config.FORMAT_AC = string.format("%s %s %s", config.FORMAT_A, string.rep(" ", config.WIDTH_B), config.FORMAT_C)
        config.FORMAT_ABx = string.format("%s %s", config.FORMAT_A, config.FORMAT_Bx)
    end

    local function DecodeInst(code)
        local iSeq, iMask, iValues = config.iABC, config.mABC, {}
        local cValue, cBits, cPos = 0, 0, 1
        for i = 1, table.getn(iSeq) do
            while cBits < iSeq[i] do
                cValue = string.byte(code, cPos) * math.ldexp(1, cBits) + cValue
                cPos = cPos + 1;
                cBits = cBits + 8
            end
            iValues[config.nABC[i]] = math.mod(cValue, iMask[i])
            cValue = math.floor(cValue / iMask[i])
            cBits = cBits - iSeq[i]
        end
        iValues.opname = config.opnames[iValues.OP]
        iValues.opmode = string.sub(config.opmode, iValues.OP + 1, iValues.OP + 1)
        if iValues.opmode == "1" then
            iValues.Bx = iValues.B * iMask[2] + iValues.C
        elseif iValues.opmode == "2" then
            iValues.sBx = iValues.B * iMask[2] + iValues.C - config.MAXARG_sBx
        end
        return iValues
    end

    local function ChunkSpy(chunk_name, chunk)
        local idx = 1
        local previdx, len
        local result = {}
        local stat = {}
        result.chunk_name = chunk_name or ""
        result.chunk_size = string.len(chunk)
        local function TestChunk(size, idx, errmsg)
            if idx + size - 1 > result.chunk_size then
                error(string.format("chunk too small for %s at offset %d", errmsg, idx - 1))
            end
        end

        local function LoadByte()
            previdx = idx
            idx = idx + 1
            return string.byte(chunk, previdx)
        end

        local function LoadBlock(size)
            if not pcall(TestChunk, size, idx, "LoadBlock") then return end
            previdx = idx
            idx = idx + size
            if config.endianness == 1 then
                return string.sub(chunk, idx - size, idx - 1)
            else
                local block = ""
                for i = idx - 1, idx - size, -1 do
                    block = block .. string.sub(chunk, i, i)
                end
                return block
            end
        end

        len = string.len(config.SIGNATURE)
        TestChunk(len, idx, "header signature")
        if string.sub(chunk, 1, len) ~= config.SIGNATURE then
            error("header signature not found, this is not a Lua chunk")
        end
        idx = idx + len
        TestChunk(1, idx, "version byte")
        result.version = LoadByte()
        if result.version ~= config.VERSION then
            error(string.format("ChunkSpy cannot read version %02X chunks", result.version))
        end
        TestChunk(1, idx, "endianness byte")
        local endianness = LoadByte()
        if not config.AUTO_DETECT then
            if endianness ~= config.endianness then
                error("unsupported endianness")
            end
        else
            config.endianness = endianness
        end
        TestChunk(8, idx, "size bytes")
        local function TestSize(mysize, sizename, typename)
            local byte = LoadByte()
            if not config.AUTO_DETECT then
                if byte ~= config[mysize] then
                    error(string.format("mismatch in %s size (needs %d but read %d)",
                        sizename, config[mysize], byte))
                end
            else
                config[mysize] = byte
            end
        end

        TestSize("size_int", "int", "bytes")
        TestSize("size_size_t", "size_t", "bytes")
        TestSize("size_Instruction", "Instruction", "bytes")
        TestSize("SIZE_OP", "OP", "bits")
        TestSize("SIZE_A", "A", "bits")
        TestSize("SIZE_B", "B", "bits")
        TestSize("SIZE_C", "C", "bits")
        TestSize("size_lua_Number", "number", "bytes")
        if config.SIZE_OP + config.SIZE_A + config.SIZE_B + config.SIZE_C
            ~= config.size_Instruction * 8 then
            error("instruction bit size total not consistent")
        end
        DecodeInit()
        if math.ldexp(1, config.SIZE_OP) < table.getn(config.opnames) then
            error("SIZE_OP too small to fit in opcodes")
        end
        TestChunk(config.size_lua_Number, idx, "number format test")
        local test_number = LoadBlock(config.size_lua_Number)
        if not config.AUTO_DETECT then
            if test_number ~= TEST_NUMBERS[config.number_type] then
                error("incorrect lua_Number format or bad test number")
            end
            config.test_number = test_number
        else
            config.test_number = nil
            for i, v in pairs(TEST_NUMBERS) do
                if not config.test_number and test_number == v then
                    config.test_number = v;
                    config.number_type = i
                end
            end
            if not config.test_number then
                error("unrecognized lua_Number type in test number")
            end
        end
        if config.AUTO_DETECT then
            config.description = nil
            for _, cfg in pairs(CONFIGURATION) do
                if cfg.endianness == config.endianness and
                    cfg.size_int == config.size_int and
                    cfg.size_size_t == config.size_size_t and
                    cfg.size_Instruction == config.size_Instruction and
                    cfg.size_lua_Number == config.size_lua_Number and
                    cfg.SIZE_OP == config.SIZE_OP and
                    cfg.SIZE_A == config.SIZE_A and
                    cfg.SIZE_B == config.SIZE_B and
                    cfg.SIZE_C == config.SIZE_C and
                    cfg.number_type == config.number_type then
                    config.description = cfg.description
                    config.MAX_STACK = cfg.MAX_STACK
                end
            end
            if not config.description then
                config.description = "chunk platform unrecognized"
            end
            if not config.MAX_STACK then config.MAX_STACK = 250 end
        end
        stat.header = idx - 1
        local function LoadFunction(funcname, num, level)
            local func = {}
            local function LoadInt()
                local x = LoadBlock(config.size_int)
                if not x then
                    error("could not load integer")
                else
                    local sum = 0
                    for i = config.size_int, 1, -1 do
                        sum = sum * 256 + string.byte(x, i)
                    end
                    if string.byte(x, config.size_int) > 127 then
                        sum = sum - math.ldexp(1, 8 * config.size_int)
                    end
                    if sum < 0 then error("bad integer") end
                    return sum
                end
            end

            local function LoadSize()
                local x = LoadBlock(config.size_size_t)
                if not x then
                    return
                else
                    local sum = 0
                    for i = config.size_size_t, 1, -1 do
                        sum = sum * 256 + string.byte(x, i)
                    end
                    return sum
                end
            end

            local function LoadNumber()
                local x = LoadBlock(config.size_lua_Number)
                if not x then
                    error("could not load lua_Number")
                else
                    local convert_func = convert_from[config.number_type]
                    if not convert_func then
                        error("could not find conversion function for lua_Number")
                    end
                    return convert_func(x)
                end
            end

            local function LoadString()
                local len = LoadSize()
                if not len then
                    error("could not load String")
                else
                    if len == 0 then
                        return nil
                    end
                    TestChunk(len, idx, "LoadString")
                    local s = string.sub(chunk, idx, idx + len - 2)
                    if len <= config.WIDTH_HEX then
                        idx = idx + len
                    else
                        while len > 0 do
                            local seg_len = config.WIDTH_HEX
                            if len < seg_len then seg_len = len end
                            len = len - seg_len
                            idx = idx + seg_len
                        end
                    end
                    return s
                end
            end

            local function LoadLines()
                local size = LoadInt()
                func.lineinfo = {}
                func.sizelineinfo = size
                for i = 1, size do
                    func.lineinfo[i] = LoadInt()
                end
            end

            local function LoadLocals()
                local n = LoadInt()
                func.locvars = {}
                func.sizelocvars = n
                for i = 1, n do
                    local locvar = {}
                    locvar.varname = LoadString()
                    locvar.startpc = LoadInt()
                    locvar.endpc = LoadInt()
                    func.locvars[i] = locvar
                end
            end

            local function LoadUpvalues()
                local n = LoadInt()
                if n ~= 0 and n ~= func.nups then
                    error(string.format("bad nupvalues: read %d, expected %d", n, func.nups))
                    return
                end
                func.upvalues = {}
                func.sizeupvalues = n
                for i = 1, n do
                    func.upvalues[i] = LoadString()
                    if not func.upvalues[i] then
                        error("empty string at index " .. (i - 1) .. "in upvalue table")
                    end
                end
            end

            local function LoadConstantKs()
                local n = LoadInt()
                func.k = {}
                func.sizek = n
                for i = 1, n do
                    local t = LoadByte()
                    if t == config.LUA_TNUMBER then
                        func.k[i] = LoadNumber()
                    elseif t == config.LUA_TSTRING then
                        func.k[i] = LoadString()
                    elseif t == config.LUA_TNIL then
                        func.k[i] = nil
                    else
                        error("bad constant type " .. t .. " at " .. previdx)
                    end
                end
            end

            local function LoadConstantPs()
                local n = LoadInt()
                func.p = {}
                func.sizep = n
                for i = 1, n do
                    func.p[i] = LoadFunction(func.source, i - 1, level + 1)
                end
            end

            local function LoadCode()
                local size = LoadInt()
                func.code = {}
                func.inst = {}
                func.sizecode = size
                for i = 1, size do
                    func.code[i] = LoadBlock(config.size_Instruction)
                    local inst = DecodeInst(func.code[i])
                    func.inst[i] = inst
                end
            end

            local start = idx
            local stat = {}
            local function SetStat(item) stat[item] = idx - start;
            start = idx
            end

            func.source = LoadString()
            if func.source == "" and level == 1 then func.source = funcname end
            func.lineDefined = LoadInt()
            if TestChunk(4, idx, "function header") then return end
            func.nups = LoadByte()
            func.numparams = LoadByte()
            func.is_vararg = LoadByte()
            func.maxstacksize = LoadByte()
            SetStat("header")
            LoadLines()
            SetStat("lines")
            LoadLocals()
            SetStat("locals")
            LoadUpvalues()
            SetStat("upvalues")
            LoadConstantKs()
            SetStat("consts")
            LoadConstantPs()
            SetStat("funcs")
            LoadCode()
            SetStat("code")
            stat.total = stat.header + stat.lines + stat.locals + stat.upvalues
                + stat.consts + stat.funcs + stat.code
            func.stat = stat
            return func
        end

        result.func = LoadFunction("(chunk)", 0, 1)
        stat.total = idx - 1
        result.stat = stat
        return result
    end

    config.AUTO_DETECT = true;
    return ChunkSpy(chunk_name, chunk);
end
