export enum TtiInstrumentModel {
    KI_2450 = "2450",
    KI_2470 = "2470",
    KI_DMM7510 = "DMM7510",
    KI_2460 = "2460",
    KI_2461 = "2461",
    KI_2461_SYS = "2461-SYS",
    KI_DMM7512 = "DMM7512",
    KI_DMM6500 = "DMM6500",
    KI_DAQ6510 = "DAQ6510",
}

export enum Ki2600InstrumentModel{
    KI_2601 = "2601",
    KI_2602 = "2602",
    KI_2611 = "2611",
    KI_2612 = "2612",
    KI_2635 = "2635",
    KI_2636 = "2636",
    KI_2601A = "2601A",
    KI_2602A = "2602A",
    KI_2611A = "2611A",
    KI_2612A = "2612A",
    KI_2635A = "2635A",
    KI_2636A = "2636A",
    KI_2651A = "2651A",
    KI_2657A = "2657A",
    KI_2601B = "2601B",
    KI_2601B_PULSE = "2601B-PULSE",
    KI_2602B = "2602B",
    KI_2606B = "2606B",
    KI_2611B = "2611B",
    KI_2612B = "2612B",
    KI_2635B = "2635B",
    KI_2636B = "2636B",
    KI_2604B = "2604B",
    KI_2614B = "2614B",
    KI_2634B = "2634B",
    KI_2601B_L = "2601B-L",
    KI_2602B_L = "2602B-L",
    KI_2611B_L = "2611B-L",
    KI_2612B_L = "2612B-L",
    KI_2635B_L = "2635B-L",
    KI_2636B_L = "2636B-L",
    KI_2604B_L = "2604B-L",
    KI_2614B_L = "2614B-L",
    KI_2634B_L = "2634B-L",
}

export enum Ki3700InstrumentModel{
    KI_3706 = "3706",
    KI_3706_SNFP = "3706-SNFP",
    KI_3706_S = "3706-S",
    KI_3706_NFP = "3706-NFP",
    KI_3706A = "3706A",
    KI_3706A_SNFP = "3706A-SNFP",
    KI_3706A_S = "3706A-S",
    KI_3706A_NFP = "3706A-NFP",
    KI_707B = "707B",
    KI_708B = "708B",
}

export enum TrebInstrumentModel {
    KI_TSPOP = "TSPop",
    KI_TREBUCHET = "Trebuchet",
}

export enum ConnectionType {
    USB = "USB",
    LAN = "LAN",
    VISA = "VISA",
}

export interface Connection {
    address: string | undefined | null
    type: ConnectionType
}

export interface Instrument {
    name: string
    description: string | undefined | null
    model: TtiInstrumentModel | Ki2600InstrumentModel | Ki3700InstrumentModel | TrebInstrumentModel
    connections: Connection[]
    available: boolean
}

export interface InstrumentList {
    instruments: Instrument[]
}

/* EXAMPLE JSON
[
    {
        "name": "2461-SYS",
        "description": "The one on my desk",
        "model": "2461-SYS",
        "connections": [
            {
                "address": "100.125.0.101",
                "type": "LAN"
            },
            {
                "type": "USB"
            }
        ],
        "available": true
    }
]
*/
