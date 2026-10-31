## ADDED Requirements

### Requirement: Tracing is opt-in and scoped to interactive connect sessions
The CLI MUST leave instrument communication and output behavior unchanged when tracing is not configured. V1 MUST support the interactive `kic connect` path only and MUST NOT trace `ping`, `check-login`, discovery, pre-terminal setup traffic, debugger binaries, or discovery binaries.

#### Scenario: Tracing is disabled by default
- **WHEN** a normal `kic` command is run without trace options
- **THEN** no trace sink is created and no trace records are emitted

#### Scenario: Tracing is enabled for a normal session
- **WHEN** a normal `kic` command is run with a valid trace destination
- **THEN** instrument communication continues through the existing connection architecture and trace events are emitted for that session

### Requirement: Successful physical reads and writes are recorded
The trace system MUST record the bytes successfully returned by a read and the bytes successfully accepted by a write at the underlying LAN, TLS, or VISA transport boundary. A read record MUST contain only the returned byte range, and a write record MUST contain only the successfully written byte range. Transport errors MUST NOT be represented as successful I/O records.

#### Scenario: A transport write succeeds
- **WHEN** the underlying transport accepts N bytes
- **THEN** one write event records exactly those N bytes and the successful byte count

#### Scenario: A transport read succeeds
- **WHEN** the underlying transport returns N bytes
- **THEN** one read event records exactly the first N bytes of the read buffer and the successful byte count

#### Scenario: A transport operation fails
- **WHEN** a read or write returns an I/O error without a successful byte count
- **THEN** no successful read or write event is emitted for that operation and the original I/O behavior is preserved

### Requirement: Local REPL commands are outside the I/O trace
The CLI MUST NOT emit payload I/O records for local REPL control commands such
as `.exit`, `.help`, `.reset`, or `.abort`. These commands are interpreted by
the REPL and are not instrument transport operations. Any instrument bytes
generated as a consequence of a command remain subject to the normal physical
I/O tracing rules, and a resulting transport close may be emitted separately
as a `connection_closed` lifecycle event.

#### Scenario: The user enters `.exit`
- **WHEN** the REPL receives the local `.exit` command
- **THEN** the command itself is not emitted as a read or write trace record
- **AND** any observed transport close is represented only by the separate lifecycle contract

### Requirement: Connection lifecycle closes are recorded separately
The interactive traced transport MUST emit at most one `connection_closed` lifecycle event when it observes an EOF or terminal transport read/write error. The lifecycle event MUST include a UTC timestamp, session ID, protocol, address when available, and a bounded reason. It MUST NOT pretend that the close is a successful read or write and MUST NOT include payload bytes.

#### Scenario: The instrument closes the transport
- **WHEN** an underlying read returns zero bytes
- **THEN** one `connection_closed` event is emitted with an EOF reason before the original close behavior is returned

#### Scenario: The transport reports a terminal error
- **WHEN** a traced read or write returns a terminal connection error
- **THEN** one `connection_closed` event is emitted with a categorized reason and the original I/O error behavior is preserved

#### Scenario: A close is observed only once
- **WHEN** cleanup follows an already reported close
- **THEN** no duplicate lifecycle event is emitted for that session transport

### Requirement: Trace events use a stable JSON Lines contract
Each trace event MUST be serialized as one complete JSON object followed by a newline. Payload I/O events MUST include a UTC timestamp, direction, session ID, protocol, connection address when available, byte length, payload encoding, and payload. Lifecycle events MUST include `event` equal to `connection_closed`, timestamp, session ID, protocol, address when available, and reason. Printable UTF-8 payloads MUST use UTF-8 encoding; payloads that cannot be represented safely as UTF-8 MUST use Base64.

#### Scenario: A text event is emitted
- **WHEN** a successful text read or write is traced
- **THEN** the destination receives one newline-delimited JSON object with `direction`, `sessionId`, `protocol`, `address`, `byteLength`, `encoding`, and `data` fields

#### Scenario: A binary event is emitted
- **WHEN** a successful payload contains bytes that are not safely represented as UTF-8
- **THEN** the event uses `encoding` equal to `base64` and `data` contains the Base64 payload

#### Scenario: A consumer receives split TCP data
- **WHEN** a JSON event arrives in multiple socket reads
- **THEN** the consumer can buffer data until the terminating newline and parse exactly one complete event without relying on transport packet boundaries

### Requirement: Session identity is propagated to every event
The CLI MUST accept a session ID from the caller when socket or file tracing is enabled and MUST include the same non-empty session ID in every event emitted for that CLI session. A new CLI invocation MUST NOT silently reuse a prior invocation's session ID.

#### Scenario: A caller supplies a session ID
- **WHEN** the extension starts a CLI session with a session ID
- **THEN** every trace event from that process contains the supplied session ID

#### Scenario: A caller omits a session ID
- **WHEN** tracing is enabled without a caller-provided session ID
- **THEN** the CLI generates a unique session ID for the process and includes it in every event

#### Scenario: Multiple connections are active
- **WHEN** two CLI sessions emit events concurrently
- **THEN** each event can be assigned unambiguously to its session using `sessionId`

### Requirement: Trace destinations are separate from normal CLI output
The CLI MUST support a dedicated trace socket destination and MAY support a dedicated trace file destination for standalone CLI diagnostics. Trace JSON objects MUST NOT be written to normal stdout or stderr as part of the trace feature, and existing diagnostic logging options MUST retain their current meaning.

#### Scenario: File tracing is configured
- **WHEN** a valid trace file destination is provided
- **THEN** trace events are appended or written as JSON Lines to that file without changing normal CLI output

#### Scenario: Socket tracing is configured
- **WHEN** a valid trace socket destination is provided and the consumer accepts the connection
- **THEN** trace events are sent as JSON Lines over that socket without changing normal CLI output

#### Scenario: Both destinations are configured
- **WHEN** file and socket tracing are both enabled
- **THEN** the same trace event is delivered independently to each configured destination

### Requirement: Trace delivery is bounded and non-blocking to instrument communication
Trace delivery MUST use a bounded asynchronous handoff between instrument I/O and destination writers. Instrument reads and writes MUST NOT wait indefinitely for file I/O, socket I/O, JSON destination processing, or a VS Code consumer. When the queue is full or a destination becomes unavailable, the CLI MUST preserve instrument communication and record a rate-limited diagnostic indication of dropped or failed trace delivery.

#### Scenario: The trace consumer is slow
- **WHEN** the trace socket or file writer cannot keep up with emitted events
- **THEN** instrument communication continues without waiting indefinitely for the trace destination

#### Scenario: The trace queue is full
- **WHEN** a new trace event cannot be queued within the configured non-blocking handoff
- **THEN** the event may be dropped, a dropped-event indication is recorded, and the instrument operation still completes according to its original behavior

#### Scenario: The trace socket disconnects
- **WHEN** the live trace consumer closes or loses the trace socket
- **THEN** the CLI stops or disables that trace destination, preserves instrument communication, and emits at most rate-limited diagnostic warnings

### Requirement: Sensitive authentication data is redacted
The trace system MUST NOT emit plaintext passwords, authentication tokens, or equivalent secrets. Redaction MUST occur before an event is handed to any trace destination, while preserving the event direction and byte-length metadata.

#### Scenario: A login payload contains a password
- **WHEN** an authentication write is traced
- **THEN** the emitted payload is redacted and the plaintext password is absent from file, socket, and diagnostic trace output

### Requirement: Trace delivery failures are isolated from the terminal session
When the trace socket is unavailable, disconnects, receives malformed data, or cannot keep up, the CLI MUST preserve instrument communication and the terminal session. The extension MUST treat malformed or unexpected JSON Lines as an isolated trace error.

#### Scenario: Trace consumer sends malformed data
- **WHEN** the extension listener receives a malformed or schema-invalid JSON line
- **THEN** it records a diagnostic indication or ignores the line and keeps the terminal session alive

### Requirement: The extension displays live trace events in one shared OutputChannel
The TSP Toolkit extension MUST create or reuse a shared `TSP Instrument I/O Trace` OutputChannel for trace display. It MUST start a localhost listener before launching the interactive terminal, pass the listener address and a unique session ID to `kic connect`, parse newline-delimited JSON incrementally, and format valid events for live display. Extension-side trace-file saving is out of scope for v1.

#### Scenario: A connection starts with tracing enabled
- **WHEN** the extension starts an interactive connection
- **THEN** it creates the listener before terminal launch and passes `--trace-socket` and `--trace-session-id` to the CLI

#### Scenario: A complete trace event arrives
- **WHEN** the listener receives one valid JSON event terminated by a newline
- **THEN** the event is displayed in the shared OutputChannel with its session ID and transport metadata

#### Scenario: One socket read contains multiple events
- **WHEN** a listener read contains multiple newline-delimited JSON events
- **THEN** each complete event is parsed and displayed in order

#### Scenario: A connection closes
- **WHEN** the interactive terminal closes or the connection flow is disposed
- **THEN** the listener is closed and its resources are released without affecting other extension commands

#### Scenario: A user opens or clears trace output
- **WHEN** the user invokes the contributed trace show or clear command
- **THEN** the shared OutputChannel is shown or cleared without creating a per-connection channel
