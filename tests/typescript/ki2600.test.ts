import { ChildProcessWithoutNullStreams, spawn } from "node:child_process"
import { env } from "node:process"
import { assert } from "chai"
import { after, afterEach, beforeEach, describe, it } from "mocha"
import { KIC_EXECUTABLE, sleep, TSPOP_EXECUTABLE } from "./util"

describe("2600B Integration", function () {
    describe("LAN Connection", function () {
        before(function () {
            //Check if any of the requisite instruments are available.
            const instruments_found = false
            if (!instruments_found) {
                this.skip()
            }
        })

        it("PLACEHOLDER", function () {
            assert.isTrue(true)
        })
    })
})
