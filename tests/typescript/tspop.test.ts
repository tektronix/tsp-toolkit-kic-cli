import { ChildProcessWithoutNullStreams, spawn } from "node:child_process"
import { env } from "node:process"
import { assert } from "chai"
import { after, afterEach, beforeEach, describe, it } from "mocha"
import { KIC_EXECUTABLE, sleep, TSPOP_EXECUTABLE } from "./util"

describe("TSPop Integration", function () {
    describe("LAN Connection", function () {
        let tsp: ChildProcessWithoutNullStreams | undefined
        ////////////////
        // Test setup //
        ////////////////
        beforeEach("Start TSPop", async function () {
            this.timeout(3000)
            env["NO_COLOR"] = "1"
            // start the tsp executable
            tsp = spawn(TSPOP_EXECUTABLE)

            await new Promise<void>((resolve) => {
                tsp?.stdout.on("data", (data) => {
                    const string_data = String(data).toString()
                    if (string_data.includes("Ready")) {
                        resolve()
                    }
                })
            })
            await sleep(100)
            // clear any errors
            tsp?.stdin.write(
                "while (true) do _,_,sev,_=errorqueue.next() if (sev==0) then break end end\n"
            )
            //"while (true) do _,msg,sev,_=errorqueue.next() if (sev==0) then print("ALL DONE") break end print(msg) end\n"
            await sleep(500)

            // send "abort\n" to allow tspop to be connected to via lan
            tsp?.stdin.write("abort\n")
        })
        //////////////////
        // Test cleanup //
        //////////////////
        afterEach("Stop TSPop", async function () {
            assert.exists(tsp)
            if (tsp) {
                tsp.stdin.write("exit")
            }
            await sleep(1000)

            tsp?.kill()
        })

        after("Make sure everything stops", function () {
            this.timeout(2000)
            tsp?.kill()
        })

        ////////////////
        // Test cases //
        ////////////////
        it("is able to connect", async function () {
            this.timeout(5000)
            const expected_re =
                /(?<idn>KEITHLEY\sINSTRUMENTS\sLLC,TSPop,0,\d\.\d\.\d.*)\n\nTSP>\s(?<msg>HELLO)\n\nTSP>\s/g
            let out = ""
            let printed = false
            let exited = false
            const kic = spawn(KIC_EXECUTABLE, ["connect", "lan", "127.0.0.1"], {
                detached: true,
            })

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
                /(?<idn>KEITHLEY\sINSTRUMENTS\sLLC,TSPop,\d,\d\.\d\.\d.*)\n\nTSP>\s(?<error>-\d\.\d+e\+\d{2}\s+TSP\sSyntax\serror\sat\sline\s2:\s`='\sexpected\snear\s`<eof>'\s+\d\.\d+e\+\d{2}\s+\d\.\d+e\+\d{2}\s+\d\.\d+e\+\d{2}\s+\d\.\d+e-\d{2})\n\nTSP>\s/g
            let out = ""
            let printed = false
            let exited = false
            const kic = spawn(KIC_EXECUTABLE, ["connect", "lan", "127.0.0.1"], {
                detached: true,
            })

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
