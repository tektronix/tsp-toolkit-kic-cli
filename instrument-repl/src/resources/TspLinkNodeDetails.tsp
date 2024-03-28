---Create the initial JSON structure
local lang_data = {
  ["$schema"] = "./tspSchema.json",
  nodes = {},
  self = ""
}

---Function to add node details dynamically
local function addNodeDetails(nodeName, model)
  lang_data.nodes[nodeName] = {
    model = model
  }
end



if tsplink.initialize == nil then
  tsplink.reset(1)
else
  tsplink.initialize(1)
end

if (tsplink.state == "online") then
  for i = 1, 64, 1 do
    if (node[i] ~= nil) then
      addNodeDetails("node" .. i, node[i].model)
    end
  end
end

lang_data.self = node[tsplink.master].model






local function tableToString(tbl)
  local str = "{"

  -- Check if the table is empty
  if next(tbl) == nil then
    return "{}"
  end

  -- Iterate over the key-value pairs
  for key, value in pairs(tbl) do
    -- Wrap the key with double quotes
    str = str .. '"' .. tostring(key) .. '": '

    -- Check if the value is a nested table
    if type(value) == "table" then
      str = str .. tableToString(value)          -- Recursive call for nested tables
    elseif value == "" then
      str = str .. '""'                          -- Empty string representation
    else
      str = str .. '"' .. tostring(value) .. '"' -- Regular string representation
    end

    str = str .. ", "
  end

  -- Remove the trailing comma and space
  str = string.sub(str, 1, -3)

  str = str .. "}"
  return str
end


-- Call the function to convert the table to a string
local str = tableToString(lang_data)
print("NODE>START", str, "NODE>END")
