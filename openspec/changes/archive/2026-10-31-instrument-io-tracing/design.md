## Context

The `kic` executable launches normal instrument sessions, while `kic-lib` owns the transport abstractions. `kic-lib/src/protocol/mod.rs` represents raw LAN/TLS and VISA protocols, `kic-lib/src/interface` defines the underlying read/write interfaces, and `AsyncStream` performs asynchronous communication through a worker thread. The CLI already has `tracing`-based diagnostic logging and a `--log-socket` mechanism, but those logs are not an exact instrument I/O stream and must retain their current meaning.

The first version feeds a live consumer in the TSP Toolkit VS Code extension and may support a CLI trace file. The extension owns the listener and presentation; this repository owns accurate event capture and delivery. The design must preserve existing session behavior when tracing is disabled and must prevent a slow or disconnected trace consumer from delaying instrument communication. V1 traces only the long-lived interactive `kic connect` process; `ping`, `check-login`, discovery, and pre-terminal setup traffic are excluded.

## Goals / Non-Goals

**Goals:**

- Capture successful physical reads and writes for normal `kic` sessions over raw LAN, TLS, and VISA.
- Keep the contract limited to communication performed by the interactive `connect` process.
- Emit a documented JSON Lines event contract with session correlation and exact byte counts.
- Support a dedicated socket destination without mixing trace records into stdout, stderr, or diagnostic logs. A CLI file destination may remain available for standalone use, but extension-side trace-file saving is deferred.
- Use bounded asynchronous delivery and best-effort failure handling.
- Redact authentication secrets before destination delivery.
- Reuse existing Rust workspace facilities and transport abstractions.

**Non-Goals:**

- Tracing `ping`, `check-login`, discovery, or pre-terminal setup traffic.
- Tracing local REPL control commands such as `.exit`, `.help`, `.reset`, and
	`.abort` as I/O records. These commands are interpreted by the CLI and are
	not themselves bytes sent to the instrument; only bytes that cross the
	instrument transport belong in the I/O trace. A transport close caused by
	`.exit` may still be represented by the separate `connection_closed`
	lifecycle record when the close is observed by the transport.
- Modifying debugger and discovery binaries.
- Adding extension-side trace-file saving, live filtering, or multiple per-connection OutputChannels.
- Tracing debugger and discovery binaries in the first version.
- Capturing network packets below the application/transport boundary.
- Reconstructing logical request/response messages from transport chunks.
- Adding live filtering, compression, remote trace authentication, automatic socket reconnection, or unbounded trace retention.

## Decisions

### 1. Add a dedicated trace subsystem instead of extending diagnostic logging

Introduce a small `kic-lib` trace module containing the event model, payload encoding, redaction boundary, and destination abstraction. The CLI will construct the enabled destination for the interactive `connect` process. Existing `tracing` layers and `--log-socket` remain diagnostic-only. The extension will consume the dedicated trace socket in a focused listener/parser component and append events to one shared OutputChannel.

This avoids changing the meaning or volume of existing logs and gives the extension a stable stream containing only trace events.

**Alternative considered:** Emit I/O data through existing `tracing` calls. Rejected because filtering, formatting, and current log consumers would make exact byte capture and a dedicated JSONL contract unreliable.

### 2. Capture at successful underlying read/write boundaries

Instrument the `Interface` implementations and VISA wrapper at the point where the underlying operation returns. The trace records only the bytes represented by the successful return value. For `AsyncStream`, capture in the worker thread around the underlying interface operations so records represent physical operations rather than only queued application requests.

The `Protocol` layer remains the common coordination point for configuration and metadata, but it must not duplicate events already emitted by the underlying transport.

**Alternative considered:** Trace only model-level `Instrument::read` and `write`. Rejected because it misses direct setup I/O and does not accurately represent partial asynchronous operations.

### 3. Use JSON Lines for the wire and file format

Serialize one complete event per line. Use UTF-8 for safely printable text and Base64 for payloads that cannot safely be represented as UTF-8. The event includes timestamp, direction, session ID, protocol, address when available, byte length, encoding, and data.

The newline delimiter allows the extension to parse records incrementally even when TCP reads split or combine records.

Successful payload operations remain `io` records. When the interactive transport observes EOF or a terminal read/write error, it emits one `connection_closed` lifecycle record with the timestamp, session ID, protocol, address, and a bounded reason string. Lifecycle records do not include direction, byte length, encoding, or payload fields.

**Alternative considered:** A single JSON array or a custom delimiter. Rejected because arrays are not naturally streamable and custom formats duplicate parsing/escaping rules.

### 4. Use a caller-provided session ID with a generated fallback

Add a CLI option for a caller-provided session ID. The extension will generate and pass one for normal connections; the CLI will generate a unique process session ID when omitted. The trace sink receives this immutable session metadata and applies it to every event.

This supports multiple active connections and makes reconnects distinguishable without coupling the CLI to extension object identities.

### 5. Use a bounded asynchronous fan-out sink

Instrument I/O creates a small owned event and performs a non-blocking enqueue into a bounded queue. A background writer serializes and sends each event to the configured file and/or socket destinations. Destination failures disable only the failed destination and produce rate-limited diagnostics. Queue overflow increments a dropped-event count and does not fail the instrument operation.

The queue size and payload limits should be constants or configuration internal to the first version; no public tuning surface is required yet. Large payloads must still be bounded to prevent a single trace event from consuming unbounded memory.

**Alternative considered:** Synchronous writes from the transport thread. Rejected because file/socket latency and a slow VS Code consumer would directly increase instrument-operation latency.

### 6. Make trace configuration explicit at CLI startup

Add dedicated global CLI options for trace file, trace socket, and session ID. Build the trace sink before dispatching the selected command so normal `kic` subcommands and process-forwarding behavior can preserve the configuration. The implementation must ensure that the existing non-VISA-to-VISA executable handoff forwards these options.

The extension generates one unique session ID per terminal connection, starts its localhost listener before `createTerminal()`, and passes `--trace-socket` and `--trace-session-id` to `kic connect`. A listener or parser failure is non-fatal to the terminal session.

### 7. Redact credentials before enqueueing

Authentication writes must be identified at the existing authentication boundary or through a trace redaction policy that can recognize protected fields. Redaction must happen before the event enters the shared queue, ensuring no destination or queued copy contains plaintext credentials. Byte length metadata may remain accurate while payload data is replaced with a fixed redacted representation.

### 7. Use one shared extension OutputChannel

The extension owns one `TSP Instrument I/O Trace` OutputChannel. Displayed records include session identity and transport metadata so concurrent connections remain distinguishable. V1 provides show and clear commands but does not save trace files from the extension.

## API Contract and Data Flow

```text
Extension listener + sessionId
	|
	| kic connect --trace-socket <address> --trace-session-id <id>
	v
kic-lib transport -> bounded async trace queue -> JSONL socket
	|
	v
extension line buffer/parser -> shared OutputChannel
```

Each socket line is one JSON object with `timestamp`, `sessionId`, `direction`, `protocol`, `address` when available, `byteLength`, `encoding`, and `data`. TCP packet boundaries are not message boundaries; the extension buffers until `\n`, handles multiple records per read, and ignores or diagnoses malformed records without terminating the terminal.

## Repository File/Component Plan

### tsp-toolkit-kic-cli

- `kic-lib`: trace event model, encoding, redaction, queue, destinations, and physical I/O capture.
- `kic`: trace options, sink construction, and option forwarding.
- Existing Rust test locations: contract, transport, queue, security, and CLI tests.

### tsp-toolkit

- A focused trace listener/parser module: localhost listener, line buffering, validation, formatting, and shared trace state.
- `src/connection.ts`: listener startup before terminal creation, session ID generation, CLI arguments, and cleanup.
- `src/extension.ts`: shared OutputChannel, show/clear commands, and disposal registration.
- `package.json`: v1 command contributions if needed; no extension trace-file export.
- `src/test`: parser, listener lifecycle, session display, and failure isolation tests.
- Lifecycle close records should be covered at the transport boundary and by extension parser/formatter tests.

## Implementation Order

1. Finalize and test the CLI JSON Lines contract and trace socket.
2. Build the CLI binaries consumed by the extension.
3. Implement the extension listener/parser and OutputChannel integration against that contract.
4. Validate CLI argument propagation and terminal lifecycle cleanup.
5. Run cross-repository tests with a fake trace producer before instrument testing.

## Risks / Trade-offs

- [Trace capture may add copying and timestamp/serialization overhead] -> Capture only successful returned bytes, enqueue once, and perform serialization/destination I/O on a worker thread. Add tests for disabled tracing and queue behavior.
- [A bounded queue can drop events during firmware or binary transfers] -> Preserve instrument communication, expose a dropped-event diagnostic, and document that the first version is best-effort under trace backpressure.
- [AsyncStream can split logical writes into physical chunks] -> Define events as physical successful transport operations and avoid promising logical message reconstruction.
- [The extension cannot read output from a VS Code terminal] -> Create a localhost trace listener before terminal launch and pass its address to the CLI.
- [Trace output can expose credentials] -> Redact before queueing and test password-containing authentication paths.
- [A socket consumer may connect late or disconnect] -> Treat the socket as an optional best-effort destination; never fail instrument communication because of its state.
- [Multiple destinations may receive events at slightly different times] -> Use the event timestamp and session ID as the correlation contract; destination delivery order is only guaranteed within each destination queue.
- [Existing CLI argument forwarding may omit new options] -> Add explicit coverage for the supported interactive connect path and verify trace options survive forwarding.

## Migration Plan

1. Implement the trace subsystem behind disabled-by-default configuration for interactive `connect`.
2. Add unit and integration tests without changing existing command behavior.
3. Build the CLI binaries consumed by the extension.
4. Integrate the extension listener, parser, OutputChannel, and terminal argument wiring.
5. Roll back by omitting trace options or reverting the CLI binary; normal sessions require no data migration.

## Open Questions

None for v1. Extension-side trace-file saving, setup-process tracing, filtering, and advanced delivery features are follow-up work.
