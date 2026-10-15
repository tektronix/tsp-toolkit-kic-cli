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
- [ ] 3.5 Keep tracing limited to the interactive `connect` transport and verify ping, check-login, discovery, and pre-terminal setup traffic are excluded.

> Deferred until CLI trace construction and forwarding are implemented in Tasks 4.1 and 4.2; this boundary check must be validated against the complete command flow.

### 4. Redaction and CLI Configuration

- [x] 4.1 Add dedicated global CLI options for trace file, trace socket, and session ID without changing the meaning of existing diagnostic logging options.
- [x] 4.2 Construct the trace socket destination before interactive command dispatch and preserve trace options through supported CLI forwarding.
- [ ] 4.3 Redact passwords, tokens, and equivalent authentication secrets before trace events enter the queue, preserving safe metadata such as direction and original byte length.
- [ ] 4.4 Keep trace JSON out of normal stdout and stderr and preserve existing behavior when no trace destination is configured.

### 5. Validation and Tests

- [ ] 5.1 Add unit tests for disabled tracing, event serialization, UTF-8/Base64 payloads, session IDs, and exact byte counts.
- [ ] 5.2 Add transport tests for successful and failed reads/writes, partial operations, raw LAN/TLS paths, VISA paths where available, and AsyncStream worker behavior.
- [ ] 5.3 Add tests for file and socket delivery, split/coalesced JSON Lines, destination disconnects, queue overflow, and bounded memory behavior.
- [ ] 5.4 Add security tests proving authentication payloads are redacted before reaching file and socket destinations.
- [ ] 5.5 Add CLI tests covering interactive trace options, normal stdout/stderr isolation, and session ID propagation.
- [ ] 5.6 Run formatting, linting, unit tests, and relevant feature-enabled tests; document any environment-dependent VISA validation.

### 6. Documentation and Handoff

- [ ] 6.1 Document the trace CLI options, JSON Lines event schema, session ID behavior, best-effort delivery semantics, and dropped-event behavior.
- [ ] 6.2 Document the socket contract for the TSP Toolkit extension, including newline framing and session expectations.
- [ ] 6.3 Confirm the v1 scope excludes ping, check-login, setup traffic, debugger/discovery tracing, packet capture, logical message reconstruction, filtering, and extension trace-file saving.

## tsp-toolkit

### 7. Listener and Event Parser

- [ ] 7.1 Add a localhost trace listener that binds before `createTerminal()` and exposes its address for CLI argument construction.
- [ ] 7.2 Implement incremental JSON Lines buffering that handles split records, coalesced records, disconnects, and malformed lines without throwing into the connection flow.
- [ ] 7.3 Validate session ID, direction, protocol, byte length, encoding, and payload fields before display; keep parser limits bounded.

### 8. Live OutputChannel Integration

- [ ] 8.1 Create or reuse one shared `TSP Instrument I/O Trace` OutputChannel and provide show and clear commands.
- [ ] 8.2 Format valid events with timestamp, session ID, direction, protocol, address, byte length, and payload for live display.
- [ ] 8.3 Generate a unique session ID per interactive connection, pass `--trace-socket` and `--trace-session-id`, and start the listener before terminal launch.
- [ ] 8.4 Close listeners and dispose connection resources when the terminal or connection flow closes; do not affect other connections or extension commands.
- [ ] 8.5 Do not add extension-side trace-file saving, per-connection OutputChannels, filtering, or setup/ping/check-login tracing in v1.

### 9. Extension Tests

- [ ] 9.1 Add parser tests for split and coalesced JSON Lines, UTF-8/Base64 data, invalid records, and bounded input.
- [ ] 9.2 Add listener lifecycle tests for startup ordering, CLI argument propagation, disconnects, cleanup, and multiple sessions sharing one OutputChannel.
- [ ] 9.3 Add failure-isolation tests proving trace listener/parser failures do not fail the terminal connection flow.

## Cross-repository validation

### 10. Contract and Integration Checks

- [ ] 10.1 Build the CLI artifact consumed by the extension and verify the exact `--trace-socket` and `--trace-session-id` arguments.
- [ ] 10.2 Run a fake localhost trace producer/consumer test covering JSONL framing, session correlation, and live OutputChannel formatting.
- [ ] 10.3 Verify trace is opt-in, stdout/stderr remain unchanged, and trace delivery or listener failures do not terminate instrument communication.
- [ ] 10.4 Run Rust formatting/lint/tests and extension tests; document environment-dependent VISA checks.
