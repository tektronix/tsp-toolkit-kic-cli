import { platform } from "os"

export const TSPOP_EXECUTABLE = (function () {
    if (platform() == "win32") {
        return "tsp.exe"
    }
    return "tsp"
})()

export const KIC_EXECUTABLE = (function () {
    const prefix = `${process.cwd()}/target/release/kic`
    if (platform() == "win32") {
        return `${prefix}.exe`
    }
    return prefix
})()

export function sleep(ms: number) {
    return new Promise<void>((resolve) => {
        setTimeout(resolve, ms)
    })
}
