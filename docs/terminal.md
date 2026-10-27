# Isolated Cube terminal

Open a workspace with `cube play`, choose a working directory and select **Start
terminal**. The console executes the real versioned `cube` binary in Cube Runner.
Try `cube encode base64 hello` or `cube --help`. Quote arguments containing spaces.
Use Ctrl+C to interrupt a command; `exit`, Ctrl+D at an empty prompt or **End session**
closes the session and its processes. Terminal input/output stays in memory and is
not automatically recorded, uploaded or added to investigation history.

The console accepts `cube ...`, `pwd`, `clear`, `exit` and
`cd <source|imports|output> [relative/path]`. It does not evaluate shell scripts,
expansions, pipelines or redirects. Use Cube's explicit output options to create
files in the output directory. The host-only `cube play` command is unavailable
inside Runner. PTY bytes are forwarded without UTF-8 conversion; terminal line
discipline still applies. The Lab never parses terminal presentation into typed
operation results.

## Working directories

| Store | Runner path | Access |
| --- | --- | --- |
| Source | `/workspace/source` | Authorized selected host directory, mounted read-only |
| Imports | `/workspace/data/imports` | Explicitly imported files, read-only |
| Output | `/workspace/output` | Separate persistent artifact volume, writable |

Directory selection accepts bounded relative paths and rejects symlinks, traversal,
Windows device names and missing directories. CLI file access is restricted by
Linux Landlock: it can read sources/imports, write output and its private temporary
directory, and load the trusted CLI/system libraries. A symlink cannot grant access
to a private target outside those roots. The data volume is mounted read-only to
Runner; **its SQLite database, notes and state are outside the CLI's Landlock
allowlist**. Only the trusted coordinator sees that parent mount. CLI processes
cannot open the database even through a source symlink.

A compatible engine must provide **Landlock ABI 3 with Landlock enabled**. Docker
Desktop enables that LSM in [4.94.0](https://docs.docker.com/desktop/release-notes/#4940)
and newer releases. Linux engine users need a compatible kernel/LSM configuration.
The launcher probes the actual runtime before startup; the runner checks again
before accepting sessions. Unsupported isolation fails explicitly. There is no
unrestricted fallback, privileged mode or container Docker socket.

## Session and resource boundaries

Each WebSocket owns one new terminal session. There is no attach-by-ID endpoint,
shared terminal or automatic reconnect to a previous process. A disconnected
browser ends that process group; reopening starts a new session. The frontend
retains bounded scrollback only while mounted and disables clipboard/hyperlink
escape handlers. It loads no remote addons or scripts.

| Resource | Bound |
| --- | --- |
| Concurrent sessions | Two per workspace |
| Idle / lifetime | Five minutes without input / 30 minutes total |
| Command wall / CPU time | 30 seconds / 20 CPU seconds |
| Runner | One CPU, 512 MiB memory, 64 processes; non-root, read-only root filesystem |
| CLI address space / output file | 256 MiB / 32 MiB |
| Input | 4 KiB/frame, 64 KiB/second, 200 control/input frames/second, 4 MiB/session |
| Output | 4 KiB/chunk, 8 MiB/session |
| Queues | 64 KiB output, 32 KiB input; slow consumers fail after two seconds |
| Temporary storage | Private per-session directory in a 16 MiB noexec/nosuid tmpfs |

The console also bounds command lines to 8 KiB/128 arguments, file descriptors to
64, disables core dumps and applies a secondary per-user process limit. Process
limits complement Docker cgroups. EOF, invalid frames, resource limits, connection
loss and shutdown terminate the session and clean child processes. Working data
and source files are retained. Docker and Landlock are defense in depth, not a
claim of an absolute boundary against a compromised host/kernel.

## Private transport and browser access

Play Local and Runner share only a private, per-workspace Unix socket volume.
Runner has `network_mode: none`, no published port, dropped capabilities and
no-new-privileges. The socket directory is `0700`, its endpoint `0600`; an owner
lock controls replacement of stale endpoints and peer credentials must match the
runtime user. Runner receives no host Docker control socket.

The browser connects to `/api/terminal/ws` with `cube-play.v1` and a separate
`cube-play.auth.<capability>` subprotocol. The server selects only the former;
the credential never enters the URL or reflected protocol. The exact Host and Origin, selected root and initial size are checked
before upgrade. Browser messages only send PTY input, resize or close; executable
paths and environment overrides are never accepted. Untrusted requests cannot
create host processes. Probe/status messages reveal only resource availability,
not commands, output, notes or host paths.

Native frontend/API development reports Runner unavailable until a compatible
Linux runner socket is explicitly configured. Windows/macOS ordinary users use
Linux containers managed by the launcher. The Vite development proxy supports
WebSockets when an authorized native Linux runner is configured.
