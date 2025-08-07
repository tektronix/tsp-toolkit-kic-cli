-- This table encodes which firmware part numbers are expected for the various
-- model numbers reported by the module. The key is the model number reported
-- by the module. The value is the expected firmware part number of the image.
-- Note that unprogrammed modules may report their model number as the firmware
-- part number.
local gFwPartTable =
{
    ["Sparta"]          = "066-2199",
    ["VTSMU-48-2"]      = "066-2199",
    ["VTSMU-48-8"]      = "066-2199",
    ["VTSMU-48-16"]     = "066-2199",
    ["VTSMU-200-1"]     = "066-2206",
    ["VTSMU-200-1-LC"]  = "066-2206",
    ["VTSMU-200-2"]     = "066-2206",
    ["VTSMU-200-2-LC"]  = "066-2206",
    ["KingArthur"]      = "066-2200",
    ["VTPSU-50-2-ST"]   = "066-2200",
    ["900071100"]       = "066-2205",   -- This is the load test module
    ["066-2199"]        = "066-2199",
    ["066-2200"]        = "066-2200",
    ["066-2205"]        = "066-2205",
    ["066-2206"]        = "066-2206",
}

-- This table encodes which firmware part numbers as reported by the firmware
-- image file are compatible with the expected firmware part number for the
-- module. That is to say, if the table above indicates firmware part number
-- 066-AAAA is the firmware to be loaded on the module, we check this table
-- using the firmware part number reported by the image as the key. If the
-- value on the right is 066-AAAA, we are good.
local gFwCompatibility =
{
    ["Sparta"]          = "066-2199",
    ["KingArthur"]      = "066-2200",
    ["066-2199"]        = "066-2199",
    ["066-2200"]        = "066-2200",
    ["066-2205"]        = "066-2205",
    ["066-2206"]        = "066-2206",
}

-- This table is used to indicate which modules use dual interfaces and
-- requires the firmware update function to program both interfaces. The
-- key is the model number.
local gSplitModules =
{
    ["VTSMU-48-16"]     = true,
    ["VTSMU-200-2"]     = true,
    ["VTSMU-200-2-LC"]  = true,
    ["900071100"]       = true,
}

checkbanks = function(lSlot, lDualBanks, lAction)
    local lLowerBank = false
    local lUpperBank = false

    if lDualBanks then
        lLowerBank = lSlot.bank[1]
        lUpperBank = lSlot.bank[2]
        if lLowerBank and lLowerBank.flash then
            lLowerBank = lLowerBank.flash
        else
            if lSlot.flash then
                print("Warning: Bank[1] not detected but slot level interface detected.")
                print("Warning: Assuming MRD image.")
                lLowerBank = lSlot.flash
            else
                print("Warning: Bank[1] not detected. Lower bank will not be " .. lAction .. ".")
            end
        end
        if lUpperBank then
            lUpperBank = lUpperBank.flash
        else
            print("Warning: Bank[2] not detected. Upper bank will not be " .. lAction .. ".")
        end
    else
        lLowerBank = lSlot.flash
    end

    return lLowerBank, lUpperBank
end

flashupdate = function(lSlot)
    local lOffset = 0
    local lSectorNumber = 1
    local lSectorAddress
    local lSectorData
    local lPart
    local lVersion
    local lDualBanks
    local lUpperBank = false
    local lLowerBank = false

    lPart = gFwCompatibility[flash.part]
    lVersion = flash.version

    if (flash.part == "MediumMrd") then
        print("Loading MRD firmware onto model " .. lSlot.model)
    elseif lPart ~= gFwPartTable[lSlot.model] then
        print("Firmware image not appropriate for module. Module: " .. lSlot.model .. ", Image: " .. lPart)
        print("Flash not updated.")
        return
    end
    print("Updating module to version " .. lVersion)

    lDualBanks = gSplitModules[lSlot.model]
    lLowerBank, lUpperBank = checkbanks(lSlot, lDualBanks, "programmed")

    errorqueue.clear()
    lSectorAddress, lSectorData = flash.sector(lSectorNumber, true)
    while lSectorAddress and (errorqueue.count == 0) do
        print('Programming sector ' .. lSectorNumber)
        if lLowerBank then
            lLowerBank.sector.load(lSectorData)
            lLowerBank.sector.program(lSectorAddress + lOffset)
        end
        if lUpperBank then
            lUpperBank.sector.load(lSectorData)
            lUpperBank.sector.program(lSectorAddress + lOffset)
        end
        lSectorNumber = lSectorNumber + 1
        lSectorAddress, lSectorData = flash.sector(lSectorNumber, true)
    end
    print("Done programming flash.")
end

flashverify = function(lSlot, lCompress)
    local lSectorNumber = 1
    local lSectorAddress
    local lSectorData
    local lRealData
    local lDualBanks
    local lUpperBank = false
    local lLowerBank = false

    errorqueue.clear()
    lDualBanks = gSplitModules[lSlot.model]
    lLowerBank, lUpperBank = checkbanks(lSlot, lDualBanks, "checked")
    lSectorAddress, lSectorData = flash.sector(lSectorNumber, lCompress)
    while lSectorAddress and (errorqueue.count == 0) do
        if lLowerBank then
            lRealData = lLowerBank.read(lSectorAddress, 32768, lCompress)
            if lRealData == lSectorData then
                print('Sector ' .. lSectorNumber .. ' (lower) same')
            else
                print('Sector ' .. lSectorNumber .. ' (lower) different')
            end
        end
        if lUpperBank then
            lRealData = lUpperBank.read(lSectorAddress, 32768, lCompress)
            if lRealData == lSectorData then
                print('Sector ' .. lSectorNumber .. ' (upper) same')
            else
                print('Sector ' .. lSectorNumber .. ' (upper) different')
            end
        end
        lSectorNumber = lSectorNumber + 1
        lSectorAddress, lSectorData = flash.sector(lSectorNumber, lCompress)
    end
end

flashencode = function (lAddress, lData)
    local lLength = string.len(lData)
    local lChunkLength
    local lString
    local lIndex
    local lByteIndex

    lByteIndex = 1
    while lLength > 0 do
        if lLength > 16 then
            lChunkLength = 16
        else
            lChunkLength = lLength
        end
        lString = string.format("S3%02X%08X", lChunkLength + 5, lAddress)
        for lIndex = 1, lChunkLength do
            lString = lString .. string.format("%02X", string.byte(lData, lByteIndex))
            lByteIndex = lByteIndex + 1
        end
        lString = lString .. "--"
        lLength = lLength - lChunkLength
        lAddress = lAddress + lChunkLength
        print(lString)
    end
end

updateSlot = function (lSlotNumber)
    if slot[lSlotNumber].firmware ~= nil and slot[lSlotNumber].firmware.update ~= nil then
        slot[lSlotNumber].firmware.verify()
        slot[lSlotNumber].firmware.update()
    end
end