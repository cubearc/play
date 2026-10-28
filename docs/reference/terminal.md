# Terminal resource and transport boundaries

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
64 and disables core dumps. A verified cgroup PID budget of at most 128 bounds
processes within the container (the runtime uses 64). Without that scoped budget,
the console applies a 128-process per-user limit. This avoids counting unrelated
host/browser threads against a container's terminal budget. EOF, invalid frames, resource limits, connection
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
before upgrade. Browser messages send PTY input, resize, close or explicit session-capture controls; executable
paths and environment overrides are never accepted. Untrusted requests cannot
create host processes. Explicit capture controls select only an ID in the current
session's private staging directory. Probe/status messages reveal only resource availability,
not commands, output, notes or host paths.

Native frontend/API development reports Runner unavailable until a compatible
Linux runner socket is explicitly configured. Windows/macOS ordinary users use
Linux containers managed by the launcher. The Vite development proxy supports
WebSockets when an authorized native Linux runner is configured.
