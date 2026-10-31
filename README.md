# Keithely Instrument Communicator 

## Instrument I/O tracing

Tracing is opt-in for the interactive `kic connect` session. Configure one or
both destinations with `--trace-file <path>` and `--trace-socket <host:port>`;
`--trace-session-id <id>` supplies the session identifier used on every
record. When it is omitted, the CLI generates a unique identifier.

Trace delivery is best effort: records use a bounded asynchronous queue, slow
or disconnected destinations are isolated, and queue overflow may drop
records without changing instrument I/O behavior. Trace JSON is never written
to normal stdout or stderr. Authentication payloads are redacted before they
enter the queue.

Each destination receives newline-delimited JSON (one object per line). A
successful payload record contains `timestamp`, `session_id`, `direction`,
`protocol`, `address`, `byte_length`, `encoding`, and `data`. Payloads use
`utf8` when safe and `base64` otherwise. A transport EOF or terminal error is
represented separately as:

```json
{"event":"connection_closed","timestamp":"...","session_id":"...","protocol":"lan","address":"...","reason":"eof"}
```

Lifecycle records contain no direction, byte count, encoding, or payload.
Only one close record is emitted per traced transport. Reasons are bounded and
categorized, for example `eof`, `read_error:connection reset`, or
`write_error:broken pipe`.

The v1 trace scope excludes `ping`, `check-login`, discovery, setup traffic,
debugger and discovery binaries, packet capture, logical message
reconstruction, filtering, and extension-side trace-file saving.

Local REPL control commands are also outside the I/O trace. Commands such as
`.exit`, `.help`, `.reset`, and `.abort` are interpreted by the CLI rather than
sent as instrument bytes, so logging them as read/write payloads would make
the transport trace misleading. Any actual instrument I/O caused by a command
is still traced at the physical transport boundary. A close resulting from
`.exit` may appear separately as a `connection_closed` lifecycle record when
the transport observes it.
