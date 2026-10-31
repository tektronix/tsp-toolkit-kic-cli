## Why

Users currently see the CLI session in a VS Code terminal, but cannot inspect the exact bytes exchanged with an instrument. Existing diagnostic logging is not a dedicated I/O trace, and the extension cannot consume output from a terminal created with `createTerminal()`. A coordinated change in the CLI and the consuming TSP Toolkit extension is required to provide live, session-correlated visibility without changing terminal behavior.

## What Changes

- Add an opt-in instrument I/O trace to normal interactive `kic connect` sessions.
- Emit structured JSON Lines trace events for successful physical reads and writes.
- Emit a structured `connection_closed` lifecycle event when the interactive transport observes a close or terminal transport error.
- Include a caller-provided session ID on every trace event so concurrent connections can be distinguished.
- Provide a dedicated trace socket contract consumed by a localhost listener in the TSP Toolkit VS Code extension.
- Provide a trace file destination for standalone CLI diagnostics.
- Add a shared `TSP Instrument I/O Trace` OutputChannel in the extension for live display.
- Have the extension create the listener before starting the terminal, pass trace options to `kic`, parse JSON Lines incrementally, and clean up when the terminal closes.
- Keep trace delivery separate from normal stdout, stderr, and existing diagnostic logging.
- Use bounded asynchronous delivery so slow trace consumers do not block instrument communication.
- Redact authentication secrets before trace events leave the CLI.
- Treat trace delivery failures, malformed events, and queue overflow as non-fatal to instrument communication or the terminal session.
- Keep lifecycle events separate from payload I/O events so a close has no misleading direction or payload byte count.
- Exclude `ping`, `check-login`, discovery, pre-terminal setup traffic, debugger/discovery binaries, packet-level tracing, logical message reconstruction, advanced filtering, multiple per-connection OutputChannels, and extension-side trace-file saving from v1.

## Capabilities

### New Capabilities

- `instrument-io-tracing`: Optional session-correlated tracing of instrument transport reads and writes with file and socket output.
- `live-instrument-io-display`: Extension-owned localhost trace listener, JSON Lines parsing, shared OutputChannel display, and session lifecycle handling.

### Modified Capabilities

None.

## Impact

- Repository `tsp-toolkit-kic-cli`: `kic-lib` for trace events, transport capture, redaction, and delivery; `kic` for trace CLI options and sink setup.
- Repository `tsp-toolkit`: `src/connection.ts` for listener lifecycle and CLI arguments; `src/extension.ts` for OutputChannel commands and disposal; `package.json` for v1 commands/settings if needed; a trace listener/parser module; and focused tests under `src/test`.
- Affected transport paths: raw LAN, TLS LAN, VISA, and asynchronous communication used by the interactive session only.
- New CLI options and a structured JSON Lines trace event contract are an integration API between the two repositories.
- No new runtime dependency should be required if the existing workspace serialization, time, threading, and socket facilities are sufficient.
- Existing behavior remains unchanged when tracing is not enabled.
