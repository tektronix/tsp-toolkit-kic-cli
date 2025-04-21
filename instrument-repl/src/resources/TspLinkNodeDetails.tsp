---Create the initial JSON structure
local newSystem =
{
  name = "",
  localNode = "",
  isActive = false,
}
local MP5103 = "MP5103"



---Function to add node
local function getNodes()
  newSystem.nodes = {}
  for i = 1, 64, 1 do
    if (node[i] ~= nil) then
      local newNode = {
        nodeId = "node" .. "[" .. i .. "]",
        mainframe = node[i].model
      }
      table.insert(newSystem.nodes, newNode)
      if (node[i].model == MP5103) then
        getSlots(node[i])
      end
    end
  end
end

---Function to add slot
local function getSlots(nodeTable)
  newSystem.slots = {}
  for i = 1, 4, 1 do
    if (nodeTable.slot[i] ~= nil) then
      local slot = {
        slotId = "slot" .. "[" .. i .. "]",
        mainframe = nodeTable.slot[i].model
      }
      table.insert(newSystem.slots, slot)
    end
  end
end


if tsplink.initialize == nil then
  tsplink.reset(1)
else
  tsplink.initialize(1)
end

if (tsplink.state == "online") then
  getNodes()
end


newSystem.localNode = node[tsplink.master].model
if (node[tsplink.master].model == MP5103) then
  getSlots(newSystem)
end



local function tableToString(tbl, orderedKeys)
  local str = ""

  -- Check if the table is an array (numeric keys)
  local isArray = next(tbl) and (function()
    for k, _ in pairs(tbl) do
      if type(k) ~= "number" then
        return false
      end
    end
    return true
  end)()

  if isArray then
    str = "["
    for _, value in ipairs(tbl) do
      if type(value) == "table" then
        str = str .. tableToString(value) .. ", "
      elseif type(value) == "boolean" then
        str = str .. tostring(value) .. ", "
      else
        str = str .. '"' .. tostring(value) .. '", '
      end
    end
  else
    str = "{"
    local processedKeys = {}
    if orderedKeys then
      -- Use orderedKeys if provided
      for _, key in ipairs(orderedKeys) do
        local value = tbl[key]
        if value ~= nil then
          str = str .. '"' .. tostring(key) .. '": '
          if type(value) == "table" then
            str = str .. tableToString(value) .. ", "
          elseif type(value) == "boolean" then
            str = str .. tostring(value) .. ", "
          else
            str = str .. '"' .. tostring(value) .. '", '
          end
          processedKeys[key] = true
        end
      end
    end
    -- Add remaining keys not in orderedKeys
    for key, value in pairs(tbl) do
      if not processedKeys[key] then
        str = str .. '"' .. tostring(key) .. '": '
        if type(value) == "table" then
          str = str .. tableToString(value) .. ", "
        elseif type(value) == "boolean" then
          str = str .. tostring(value) .. ", "
        else
          str = str .. '"' .. tostring(value) .. '", '
        end
      end
    end
  end

  -- Remove the trailing comma and space
  str = string.sub(str, 1, -3)

  -- Close the JSON structure
  if isArray then
    str = str .. "]"
  else
    str = str .. "}"
  end

  return str
end


-- Call the function to convert the table to a string
local orderedKeys = { "name", "localNode", "isActive", "slots", "nodes" }
local str = tableToString(newSystem, orderedKeys)
print("NODE>START", str, "NODE>END")
