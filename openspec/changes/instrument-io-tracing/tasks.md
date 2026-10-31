## tsp-toolkit-kic-cli

### 1. Trace Contract and Core Types

- [x] 1.1 Add a `kic-lib` trace module defining direction, protocol metadata, session metadata, payload encoding, and JSON Lines event serialization.
- [x] 1.2 Implement UTF-8 payload handling with Base64 fallback and ensure byte length is measured from the original payload.
- [x] 1.3 Add session ID creation and propagation with a caller-provided value plus a unique generated fallback for interactive `connect`.

### 2. Bounded Trace Delivery

- [x] 2.1 Implement a bounded, thread-safe event queue and background destination worker that does not block instrument I/O indefinitely.
- [x] 2.2 Implement file delivery as newline-delimited JSON and socket delivery as the same newline-delimited event stream.
- [x] 2.3 Implement independent destination failure handling, queue-overflow accounting, and rate-limited diagnostic reporting without returning trace failures as instrument I/O errors.
- [x] 2.4 Bound event payload or queued memory usage so a large transfer cannot create unbounded trace memory consumption.

### 3. Transport Instrumentation

- [x] 3.1 Add optional trace context to the normal `kic-lib` connection construction while preserving the no-trace path and existing public behavior.
- [x] 3.2 Record successful raw LAN and TLS reads and writes at the underlying transport boundary using only the successful byte range.
- [x] 3.3 Record successful VISA reads and writes at the VISA wrapper boundary using the same event contract.
- [x] 3.4 Record physical reads and writes performed by `AsyncStream` without duplicating events at higher layers.
- [x] 3.5 Keep tracing limited to the interactive `connect` transport and verify ping, check-login, discovery, and pre-terminal setup traffic are excluded.

> Verified after CLI trace construction and forwarding: local trace context is created only for interactive `connect`; non-connect handlers and setup paths retain the no-trace connection flow.

### 4. Redaction and CLI Configuration

- [x] 4.1 Add dedicated global CLI options for trace file, trace socket, and session ID without changing the meaning of existing diagnostic logging options.
- [x] 4.2 Construct the trace socket destination before interactive command dispatch and preserve trace options through supported CLI forwarding.
- [x] 4.3 Redact passwords, tokens, and equivalent authentication secrets before trace events enter the queue, preserving safe metadata such as direction and original byte length.
- [x] 4.4 Keep trace JSON out of normal stdout and stderr and preserve existing behavior when no trace destination is configured.

### 5. Validation and Tests

- [x] 5.1 Add unit tests for disabled tracing, event serialization, UTF-8/Base64 payloads, session IDs, and exact byte counts.
- [x] 5.2 Add transport tests for successful and failed reads/writes, partial operations, raw LAN/TLS paths, VISA paths where available, and AsyncStream worker behavior.

> Deterministic raw and AsyncStream boundary tests cover successful partial I/O, failed operations, and physical worker reads/writes. Live LAN/TLS endpoint and VISA hardware validation remain environment-dependent.
- [x] 5.3 Add tests for file and socket delivery, split/coalesced JSON Lines, destination disconnects, queue overflow, and bounded memory behavior.

> CLI-side coverage verifies JSONL records remain individually parseable when coalesced and that disconnected sockets do not stop file delivery. Incremental split/coalesced socket parsing remains an extension-side responsibility.
- [x] 5.4 Add security tests proving authentication payloads are redacted before reaching file and socket destinations.
- [x] 5.5 Add CLI tests covering interactive trace options, normal stdout/stderr isolation, and session ID propagation.
- [x] 5.6 Run formatting, linting, unit tests, and relevant feature-enabled tests; document any environment-dependent VISA validation.

> Formatting, `kic` and `kic-lib` unit tests, VISA feature compilation, OpenSpec validation, and diff checks pass. Strict Clippy remains blocked by pre-existing workspace lint failures, including 38 diagnostics in `kic-lib`; live VISA hardware validation was not available in this environment.

### 6. Documentation and Handoff

- [x] 6.1 Document the trace CLI options, JSON Lines event schema, session ID behavior, best-effort delivery semantics, and dropped-event behavior.
- [x] 6.2 Document the socket contract for the TSP Toolkit extension, including newline framing and session expectations.
- [x] 6.3 Confirm the v1 scope excludes ping, check-login, setup traffic, debugger/discovery tracing, packet capture, logical message reconstruction, filtering, and extension trace-file saving.

## tsp-toolkit

### 7. Listener and Event Parser

- [x] 7.1 Add a localhost trace listener that binds before `createTerminal()` and exposes its address for CLI argument construction.
- [x] 7.2 Implement incremental JSON Lines buffering that handles split records, coalesced records, disconnects, and malformed lines without throwing into the connection flow.
- [x] 7.3 Validate session ID, direction, protocol, byte length, encoding, and payload fields before display; keep parser limits bounded.

### 8. Live OutputChannel Integration

- [x] 8.1 Create or reuse one shared `TSP Instrument I/O Trace` OutputChannel and provide show and clear commands.
- [x] 8.2 Format valid events with timestamp, session ID, direction, protocol, address, byte length, and payload for live display.
- [x] 8.3 Generate a unique session ID per interactive connection, pass `--trace-socket` and `--trace-session-id`, and start the listener before terminal launch.
- [x] 8.4 Close listeners and dispose connection resources when the terminal or connection flow closes; do not affect other connections or extension commands.
- [x] 8.5 Do not add extension-side trace-file saving, per-connection OutputChannels, filtering, or setup/ping/check-login tracing in v1.

### 9. Extension Tests

- [x] 9.1 Add parser tests for split and coalesced JSON Lines, UTF-8/Base64 data, invalid records, and bounded input.
- [x] 9.2 Add listener lifecycle tests for startup ordering, CLI argument propagation, disconnects, cleanup, and multiple sessions sharing one OutputChannel.
- [x] 9.3 Add failure-isolation tests proving trace listener/parser failures do not fail the terminal connection flow.

## Cross-repository validation

### 10. Contract and Integration Checks

- [x] 10.1 Build the CLI artifact consumed by the extension and verify the exact `--trace-socket` and `--trace-session-id` arguments.
- [x] 10.2 Run a fake localhost trace producer/consumer test covering JSONL framing, session correlation, and live OutputChannel formatting.
- [x] 10.3 Verify trace is opt-in, stdout/stderr remain unchanged, and trace delivery or listener failures do not terminate instrument communication.
- [x] 10.4 Run Rust formatting/lint/tests and extension tests; document environment-dependent VISA checks.

> Final validation: Rust formatting passes; `kic` unit tests pass (3 tests); `kic-lib` unit tests pass (55 tests); and the extension compile, lint, and test pipeline passes (64 tests).
>
> Strict Clippy with `-D warnings` remains blocked by pre-existing workspace diagnostics, including 37 `kic-lib` diagnostics. The configured `rustdoc::missing_doc_code_examples` lint is unstable on the installed toolchain, so the aggregate `cargo test -p kic -p kic-lib` command's doctest phase is environment-blocked even though all unit tests pass. Live VISA hardware validation was unavailable; deterministic VISA parsing and feature compilation remain the available checks.

### 11. Connection Lifecycle Events

- [x] 11.1 Extend the CLI trace contract with a separate `connection_closed` lifecycle event and bounded close reasons.
- [x] 11.2 Emit one lifecycle event from traced LAN/TLS, VISA, and AsyncStream terminal close/error paths without changing original I/O errors.
- [x] 11.3 Accept, validate, format, and test lifecycle events in the extension listener and shared OutputChannel.
- [x] 11.4 Run focused Rust, extension, and cross-repository validation for lifecycle events and document the updated contract.
