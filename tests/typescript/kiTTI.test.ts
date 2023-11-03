import { spawn } from "node:child_process"
import * as fs from "node:fs"
import * as path from "node:path"
import * as http from "node:http"
import { env } from "node:process"
import { assert } from "chai"
import { beforeEach, describe, it } from "mocha"
import * as dotenv from "dotenv"
import { KIC_EXECUTABLE } from "./util"
import { ConnectionType, Instrument, TtiInstrumentModel } from "./config"

// eslint-disable-next-line @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-call
dotenv.config()

let instruments_found: Instrument | null = null
describe("TTI Integration", function () {
    describe("LAN Connection", function () {
        before(async function () {
            env["NO_COLOR"] = "1"
            // load local config JSON
            const config_path = path.normalize(env["KIC_TEST_CONFIG"] ?? "")
            const config: string = fs.readFileSync(config_path).toString()
            const instruments: Instrument[] = JSON.parse(config) as Instrument[]
            // filter insturments by known TTI models and availability
            const filtered = instruments.filter((inst: Instrument) => {
                if (
                    inst.available &&
                    Object.values<string>(TtiInstrumentModel).includes(
                        inst.model.toString()
                    )
                ) {
                    inst.connections = inst.connections.filter((conn) => {
                        return conn.type === ConnectionType.LAN
                    })
                    return inst.connections.length >= 1
                }
                return false
            })
            // ping the remaining instruments to see which ones are viable
            instruments_found = await new Promise<Instrument | null>(
                (resolve) => {
                    for (const instr of filtered) {
                        for (const conn of instr.connections) {
                            const req_options = {
                                host: conn.address,
                                path: "/lxi/identification",
                                method: "GET",
                            }
                            http.request(req_options, (response) => {
                                response.on("data", () => {
                                    const ret_inst = instr
                                    ret_inst.connections = [conn]
                                    instr.available = false
                                    resolve(ret_inst)
                                })
                                response.on("error", () => {
                                    resolve(null)
                                })
                            }).end()
                        }
                    }
                    return null
                }
            )

            // Take the first one and mark it as "not available", set insturment_found as true
            if (instruments_found === null) {
                this.skip()
            }
        })

        beforeEach("Clear Instrument", function () {
            //TODO
            console.log("TODO: Clear instrument")
        })

        it("is able to connect", async function () {
            this.timeout(5000)
            const expected_re =
                /(?<idn>KEITHLEY\sINSTRUMENTS,MODEL\s+.*,\d*,\d\.\d\.\d.*)\n\nTSP>\s(?<msg>HELLO)\n\nTSP>\s/g
            let out = ""
            let printed = false
            let exited = false
            const kic = spawn(
                KIC_EXECUTABLE,
                [
                    "connect",
                    "lan",
                    instruments_found?.connections[0].address ?? "",
                ],
                {
                    detached: true,
                }
            )

            kic.stdout.on("data", (data) => {
                const string_data = new String(data)
                out += string_data
                if (string_data.includes("TSP> ")) {
                    if (!printed) {
                        printed = true
                        kic.stdin.write("print([[HELLO]])\n")
                    } else if (!exited) {
                        exited = true
                        kic.stdin.end(".exit\n")
                        kic.kill()
                    }
                }
            })

            await new Promise<[number | null, NodeJS.Signals | null]>(
                (resolve) => {
                    kic.on("exit", (code, sig) => {
                        resolve([code, sig])
                    })
                    kic.on("close", (code, sig) => {
                        resolve([code, sig])
                    })
                }
            )
            assert.isTrue(expected_re.test(out))
        })

        it("is able to get Errors", async function () {
            this.timeout(5000)
            const expected_re =
                /(?<idn>KEITHLEY\sINSTRUMENTS,MODEL\s+.*,\d*,\d\.\d\.\d.*)\n\nTSP>\s(?<error>-\d+\s+TSP\sSyntax\serror\sat\sline\s\d+:\s`='\sexpected\snear\s`<eof>'\s+\d+\s+\d+\s+\d+\s+\d+)\n\nTSP>\s/g
            let out = ""
            let printed = false
            let exited = false
            const kic = spawn(
                KIC_EXECUTABLE,
                [
                    "connect",
                    "lan",
                    instruments_found?.connections[0].address ?? "",
                ],
                {
                    detached: true,
                }
            )

            kic.stdout.on("data", (data) => {
                const string_data = new String(data)
                out += string_data
                if (string_data.includes("TSP> ")) {
                    if (!printed) {
                        printed = true
                        kic.stdin.write("x\n")
                    } else if (!exited) {
                        exited = true
                        kic.stdin.end(".exit\n")
                        kic.kill()
                    }
                }
            })
            await new Promise<[number | null, NodeJS.Signals | null]>(
                (resolve) => {
                    kic.on("exit", (code, sig) => {
                        resolve([code, sig])
                    })
                    kic.on("close", (code, sig) => {
                        resolve([code, sig])
                    })
                }
            )
            assert.isTrue(expected_re.test(out))
        })
    })
})
