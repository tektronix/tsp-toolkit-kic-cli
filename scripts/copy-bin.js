#!/usr/bin/env node
// @ts-check
const fs = require("fs/promises")
const fs_constants = require("fs").constants
const path = require("path")

// First argument is "debug" or "release".
const PROFILE = process.argv[2]

const _BINARY_BASENAME = "kic"
const _BINARY2_BASENAME = "echo_socket_server"
const SRC_NAME = _BINARY_BASENAME + (process.platform === "win32" ? ".exe" : "")
const SRC2_NAME = _BINARY2_BASENAME + (process.platform === "win32" ? ".exe" : "")
// XXX: Does macOS care about extensions? Hopefully not.
//      Ran a basic test with a shell script, but a more robust test is necessary.
const DEST_NAME = _BINARY_BASENAME + ".exe"
const DEST2_NAME = _BINARY2_BASENAME + ".exe"

const rootDir = path.normalize(path.join(__dirname, ".."))
const sourceFile = path.join(rootDir, "target", PROFILE, SRC_NAME)
const source2File = path.join(rootDir, "target", PROFILE, SRC2_NAME)
const destination = path.join(rootDir, "bin")
const destinationFile = path.join(destination, DEST_NAME)
const destination2File = path.join(destination, DEST2_NAME)

const exit = function (reason) {
    console.error(reason)
    process.exit(1)
}
const copy = function () {
    fs.copyFile(sourceFile, destinationFile).catch(exit)
    fs.copyFile(source2File, destination2File).catch(exit)
}

fs.access(destination, fs_constants.R_OK | fs_constants.W_OK)
    .then(copy)
    .catch(() => {
        fs.mkdir(destination).then(copy).catch(exit)
    })
