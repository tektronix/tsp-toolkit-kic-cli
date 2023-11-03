const path = require("path")
const os = require("os")

const EXTENSION = (() =>{
    if (os.platform() === "win32") {
        return `.exe`
    } else {
        return ""
    }
})()

const PATH = path.join(__dirname, "bin")

const NAME = `kic${EXTENSION}`
const EXECUTABLE = path.join(PATH, NAME)

const DISC_NAME = `kic-discover${EXTENSION}`
const DISC_EXECUTABLE = path.join(PATH, DISC_NAME)

module.exports = {
    NAME,
    PATH,
    EXECUTABLE,
    DISC_EXECUTABLE
}
