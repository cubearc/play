# Desktop IPC and native execution

The desktop boundary connects a packaged React renderer to a Rust engine through
Electron's privileged main process. This page owns that boundary; the
[domain contract](desktop-domain.md) owns records and evidence. Delivery and
unresolved platform evidence remain in [the native contract issue](https://github.com/cubearc/play/issues/58).

## Implemented development proof

`npm run desktop:build --prefix frontend` builds `play-engine`, the real sibling Cube CLI, the packaged
renderer resources and the preload. `npm run desktop:proof --prefix frontend`
opens the actual Electron application. No web server, Docker or browser launcher
participates in this path. The [notebook preview](../development.md#explore-the-notebook-preview)
uses this boundary for managed notebooks. The engine persists Workspace content
and reasoning in [native storage](desktop-storage.md), and resolves exact owned
text Asset revisions for Core execution in a separate worker. Graph
[PipelineRuns](desktop-runs.md) preserve accepted configuration, actual attempts
and the chosen private output retention. Credential storage is not implemented.

The Electron proof executes on macOS with a locally ad-hoc-signed App Sandbox
helper and on Linux x64/ARM64 with mandatory Landlock and seccomp. Both use the
same engine, Core registry and Run supervisor. The Windows path rejects execution
with `native_policy_unavailable`. No path has an unrestricted or
container fallback. This is not a release-supported desktop
distribution. The complete application still requires the remaining shell, storage, worker and
packaging deliveries. The interactive native PTY below supports macOS and Linux development builds.

Graph execution discovers the pure executors registered in Core. Descriptors
supply operation names, typed ports, parameters and representations; Play has no
per-operation execution switch. The registered capability must support the host's
bounded representations and retention policy before it is offered. The separate
unbound development probe uses ROT13 and Brainfuck; it is not the managed
Workspace execution API. See [execution capabilities](desktop-pipelines.md#execution-capabilities).

## Trust and process topology

The target desktop topology has one Electron main process per user application
instance, with one Rust engine per open Workspace identity. Multiple windows
must attach to the same owner engine, never independent writable SQLite
connections. The implemented preview has one trusted window and one active
Workspace; multiwindow and external CLI attachment remain separate deliveries.
The engine owns domain authorization, its private store and assets. In the target
topology, workers and native PTYs belong to its execution supervisor. Algorithms
and operation metadata belong to Core.

Main starts the exact bundled engine through inherited private stdin/stdout
pipes. These are not discoverable TCP services. The proof verifies build-time
binary digests and the helper's macOS code signature before startup. Digests bind
local build artifacts together; they are not publisher authentication. Release
packaging must authenticate app, engine, CLI and workers before launching them.

The managed Workspace engine owns an OS-held exclusive lock on its
canonical private store, including its Workspace ID and storage generation. A
second process contacts that owner or rejects opening the store. A PID or lock
file is not ownership: after a crash, only acquiring the OS lock permits recovery.
Never remove a live lock because of an elapsed timeout or a reused PID.

The target external CLI discovers a per-Workspace native endpoint through a private
per-user registry. Unix sockets require a user-owned private directory; Windows
named pipes require a DACL for that user's SID and rejection of remote clients.
Peer authentication, a session credential and exact Workspace/generation binding
are mandatory before contextual commands. Do not place credentials in terminal
arguments, command logs, source roots or readable URL parameters. There is no
localhost HTTP discovery fallback. External CLI attachment is not implemented by
this development slice. Workspace locking
and crash release are exercised through real engine processes.

## Renderer authority and resources

The trusted top frame loads `cube-play://app/index.html`. Main verifies both the
registered `webContents` object and its current `mainFrame` object, then compares
the exact document URL. Matching a hostname or loading that URL in another window
does not grant authority. Navigation does not preserve authorization.

The renderer has sandboxing and context isolation enabled, Node integration and
webviews disabled. The preload exposes `catalog()`, scoped `workspace(intent)`,
`importImage(intent)`, `image(assetRef)`, `startRun(intent)`,
`cancelRun(context, runId)` as one typed object, `terminal(intent)` and `dirty(boolean)`. Closed request shapes and UTF-8
limits are checked in preload before forwarding across IPC; main and Rust check
their own boundaries again. No raw `ipcRenderer`, arbitrary channel, path, Node,
filesystem, database or shell API crosses the bridge.

Packaged resources use an exact allowlist of `index.html`, `app.js`, `app.css` and
the bundled Codicons font `app.ttf`.
The protocol handler never converts a request pathname into a filesystem path.
Unknown names, hosts, methods, credentials, query parameters and fragments are
rejected. Encoded traversal cannot expand the resource allowlist. A restrictive
CSP denies network connections, frames, forms, inline scripts and object content;
font loading is restricted to the same packaged origin. A per-app random style
nonce authorizes only xterm style elements created synchronously while opening
the trusted terminal. Other inline CSS remains blocked; no unsafe-inline or
external stylesheet permission is added.
Permissions, popups and renderer navigation are denied.

Workspace asset reads must resolve an engine-authorized Asset revision to a
short-lived resource grant bound to the requesting window and context. Export
uses a native save intent followed by an explicit engine output grant. Neither
surface accepts renderer-provided absolute paths or borrows packaged-resource
authority. PNG reads use exact random `cube-play://app/assets/<token>` URLs with
a 60-second lifetime, at most 32 grants and revocation on context/window change.
Main binds them to its trusted window/current top frame and Exploration/Stage.
Another window cannot reuse the URL. The handler returns only bounded PNG bytes
with no-store caching. Export grants are not implemented.

## Versioned private protocol

[`desktop/protocol.json`](https://github.com/cubearc/play/blob/main/desktop/protocol.json)
is the shared source of protocol/version and byte/queue/deadline limits. Rust
embeds it; main reads it; the preload bundle includes its input limit. Rust tests
require its engine version to match the built Cargo package. Electron is pinned
in the npm lockfile, Rust in `rust-toolchain.toml`, and Node in `frontend/.nvmrc`.

Frames are one UTF-8 JSON object followed by LF. The frame budget includes LF.
Both directions enforce the byte ceiling. Rust rejects duplicate keys, excessive
nesting/nodes, unknown fields and unknown command tags before dispatch. Request
IDs are positive, increasing session-local `u32` values. Main assigns them; the
renderer cannot target another invocation by guessing an ID.

| Command | Required fields in addition to `type` and `id` | Response |
| --- | --- | --- |
| `hello` | `protocol`, exact `engine` version, 32-hex-character `nonce` | `ready` echoes all three values. |
| `catalog` | None | `catalog` with adapter `operations` from Core descriptors. |
| `workspace` | Closed `command` intent | `workspace` with scoped domain data. |
| `cli_read` | Exact `context` and closed Asset/Pipeline `command` | `workspace` with scoped read data. |
| `start_cli_run` | Closed Run `request` | `run` from the same coordinator with the CLI caller. |
| `terminal` | Closed `open`, `read`, `write`, `resize` or `close` intent | `terminal` with session state, bounded Base64 output and optional Run/Trail refresh identities |
| `start_run` | Closed Run `request` | `run` containing the accepted or idempotently recovered record. |
| `cancel_run` | Exact `context`, `run_id` UUID | `run` after killing/reaping and recording cancellation, or the original terminal record. |
| `invoke` | `invocation`: `operation`, `operation_version`, `input`, `parameters` | Unbound native capability probe only; managed engine rejects `scoped_invocation_required`. |
| `cancel` | Session-local `run_id` | Unbound capability probe only; managed engine rejects `scoped_cancel_required`. |

Protocol **20** uses a 262144-byte frame ceiling, including bounded Base64 PNG/file
imports. The renderer has no literal invocation, old asset-execution alias or raw
image/file-import bridge. File chooser intents carry only `context` and `key`;
Main validates the trusted sender, selects and reads one bounded regular file,
then sends an internal `add_file` command. Renderer paths, URLs, raw file bytes
and extra fields are refused. Preview text never becomes HTML or a resource URL.
Workspace intents cover list/create/open, create/open Exploration, create Stage,
add text/Page source, revise existing text/captions/Pages, reorder, duplicate, archive or restore Stage content Blocks,
save retained Run outputs as Blocks, add annotations, save individual Notes, list scoped Run summaries
and read Run details. Internal image import/read and file import commands are Main-owned.
Every contextual request carries Workspace, Exploration and Stage UUIDs; the
engine resolves content and rejects mixed owners or absent exact revisions.
Stage responses expose one tagged active Block sequence, including reasoning,
plus explicit archived records. Trail views expose their derived `archived` state
and exact named Asset binding snapshots. Pipeline configuration uses complete
format-4 definitions; `save_trail` supplies additional input references separately.
Run admission requires the expected Trail revision and explicit target node.
`reorder_blocks` pins the Stage revision and every Block UUID; see
[Block ordering](desktop-storage.md#block-ordering) for conflict and identity rules.
`duplicate_block` pins that structural revision plus the primary and optional
caption Asset IDs/revisions, or the tagged annotation Note ID/revision.
Only the current closed `asset` or `note` source shapes are accepted. [Copy provenance](desktop-storage.md#block-copies-and-provenance)
defines the independent identities, transaction and immutable origin record.
`archive_block` and `restore_block` use that same exact Block/source snapshot and
Stage revision. [Block lifecycle](desktop-storage.md#block-archive-and-restoration)
defines retention, restoration and execution admission.
`pin_run_output` names the exact Run, attempt and port plus the expected Stage
revision. [Saved outputs](desktop-storage.md#saved-run-outputs) defines typed
value resolution, immutable origins, unique results and retry behavior. Asset
origins are tagged `asset` or `run_output`; old untagged payloads are not accepted.
The `revise_text` intent pins its Block/Asset IDs and expected revision. Run
requests require `source_asset_id` and `source_revision` as well as the selected
Pipeline revision; stale bindings fail before admission. Prior protocol shapes
are refused, without optional-field defaults or a compatibility command.
Managed `run` replies acknowledge durable admission immediately. Clients query
recorded progress/detail; the engine commits final state independently and never
sends unsolicited transport replies. The unbound `invoke`/`cancel` entry remains
only for native capability tests and is absent from the renderer bridge.

[Pipeline commands](desktop-pipelines.md#closed-commands) create, revise, fork and
read exact definitions. `save_trail` carries its expected revision, hypothesis and
nullable Pipeline reference. All fields are required at the desktop boundary;
prior payload shapes are refused. Worker invocations include saved parameters.

The native catalog reports each adapter's full Core contract, input semantic type
and effective default parameters. The editor presents compatible named ports
for ROT13, Brainfuck, image pixel reads, UTF-8 encoding/decoding and ordered
text/byte joins, JSON parsing/field selection, explicit string/integer conversion,
image dimensions and exact integer maximum/addition. Conversion and composition
algorithms remain in Core. JSON is validated against the full consumer schema;
its serialized value counts toward native input/output budgets. Byte
literals use explicit Base64, and the text join editor exposes a Separator field.
The editor saves complete graphs in immutable revisions. Rust owns dependency planning;
the worker receives only that selected closure, the bounded source bytes and
explicit literals, with no database or filesystem authority. Private node progress
frames let the engine persist each actual attempt/result before final completion. Image defaults are `x: 0`, `y: 0`,
`expand_palette: false`; saved revisions can choose supported coordinates. Rust resolves
and verifies the authorized PNG revision, validates decoded `image.samples`
against Core's named schema and freezes the parameters in the Run. The isolated
worker receives bounded Base64 PNG bytes through its inherited pipe, never an
Asset path or database handle. Its image-specific bound is the encoded form of
the existing 128 KiB image ceiling; the 8192-byte text invocation ceiling and
262144-byte frame ceiling, 16384-byte output ceiling and worker limits remain unchanged. Samples come from Core;
named byte outputs use the same retained result contract as Brainfuck.

The first command must be `hello` within the handshake deadline. A mismatched
binary/protocol or malformed first frame closes the session. The nonce binds the
reply to this inherited pipe; it is not a substitute for authenticating external
CLI peers. There is no older-version negotiation or fallback interpreter.

`finished` has `status` of `completed`, `failed` or `cancelled`; only completed
responses contain an output. Rejections have a bounded machine-readable `code`.
Diagnostics do not echo input, paths or stderr. Unscoped diagnostic invocations
retain one active probe. Managed Workspaces instead admit two active Runs and four
queued Runs, with one active and two queued per Exploration. The inbound command
queue is separate from this execution queue. Each Pipeline worker runs at most
two ready nodes under its own process CPU/VM and Run wall-time/output budgets.
See [Run admission and aggregate limits](desktop-runs.md#concurrent-runs-and-queue).
The renderer does not schedule operation dependencies.

Cancellation kills and waits for the owned worker before announcing its terminal
state. The private `started` frame includes the worker PID for supervisor
diagnostics; the preload never forwards it. A native test verifies that this PID
is gone while the engine remains alive after acknowledging cancellation.
EOF and malformed subsequent frames drop the supervisor and reap its
active child. The supervisor enforces the run wall deadline. Main rejects pending
calls on transport failure/timeout, closes the pipe, then kills an unresponsive
engine after a short shutdown grace. It never reports interrupted work as success.

The worker receives one newline-terminated task through a private pipe. The
engine keeps its write end open for the worker's lifetime. A worker thread watches
that channel: EOF, a read failure or any additional byte exits the process with
failure, including while the main thread is executing Core code. A failed watcher
prevents execution. The kernel closes the engine's end after an abrupt exit such
as `SIGKILL`; cleanup does not depend on an engine destructor or a parent-PID poll.
The same bounded frame parser rejects oversized, unterminated and malformed tasks.
This replaces the former EOF-terminated worker input; paired binaries must match.

## Lifecycle and recovery

Closing a window revokes that frame; the application and engine keep supervising
accepted Runs while alive. Activation opens another trusted window.
Unsaved Note content prompts for keeping the editor open or discarding its draft.
Saved content survives explicit Quit, which closes SQLite before releasing its
OS lock and waits for engine shutdown. An active Run is killed/reaped and
committed as cancelled on private-session closure. The complete
Workspace application must detach windows while preserving engine-owned Runs,
and implement its chosen explicit background-work policy without depending on a
React component remaining mounted.

A crashed renderer loses all resource grants. A restarted engine starts a fresh
authenticated session and checks the store generation under its owner lock before
reconciliation. Unfinished persisted Runs become interrupted; completed evidence
is immutable. Updates drain/cancel workers and PTYs, release ownership, check the
new binary/store contract, then transfer control. No two engine versions may write
the store together. The development app has no automatic engine restart or updater.

## Native capability contract and evidence

Renderer sandboxing does not sandbox Rust. Native execution must prove these
capabilities on each supported OS and architecture before desktop release.

| Boundary | macOS development proof | Complete native desktop requirement |
| --- | --- | --- |
| Private/source files | App Sandbox helper receives bytes through stdin; private sentinel reads/writes are denied. | Authorized sources are read-only; no access to engine SQLite, credentials or unrelated source roots. |
| Outputs | JSON stdout is bounded; the helper may use its own sandbox container. No host export grant exists. | Explicit export grants, private scratch quotas and no implicit source overwrite. |
| Network | No network entitlements; an actual loopback connection is denied. | Workers and PTYs cannot connect or listen outside the stated policy. |
| CPU and time | Kernel CPU limit plus engine wall deadline. | Per-worker and aggregate Run/PTY limits, with bounded admission and cancellation. |
| Memory | Core work budgets and a kernel limit on additional VM mappings after the runtime baseline. | Define and prove total/aggregate memory accounting; VM growth is not an RSS/physical-memory ceiling. |
| Children | Worker-local `RLIMIT_NPROC=0`; an actual spawn is denied and the parent remains unaffected. | Owned process-tree tracking and termination for workers and real PTYs. |
| Output/handles | Bounded wire reads/writes and descriptor/file-size limits. | Aggregate scratch, retained output, PTY stream and handle ceilings. |
| Crash cleanup | Cancel/session closure kills and reaps the worker; the worker exits on engine-pipe loss. A real engine `SIGKILL` test verifies that the owned PID disappears and the Run recovers as interrupted. | Prove app crash, grandchildren and PTY lifetime on the native OS. |

macOS uses the supported App Sandbox entitlement on a signed helper bundle, not
`sandbox-exec` or private Seatbelt initialization. Its initial runtime/shared-cache
mappings are measured through the safe `libproc` API before setting the additional
VM allowance. Refusing the memory policy aborts the worker; no failed limit is
silently ignored. Core's own Brainfuck work/memory/output limits also apply.

### Linux worker boundary

The native worker proof runs directly on Ubuntu 24.04 x64 and ARM64 hosts. It
requires an enabled Landlock LSM with ABI 3 or newer, seccomp filter support and
readable `/proc/self/status` at startup. Kernel version alone does not establish
that these facilities are enabled. Unsupported architectures or a failed policy
refuse execution before reading a task; there is no weaker fallback.

The single-threaded worker measures its initial virtual address space, imposes
hard resource limits, then installs a Landlock ruleset with no filesystem grants
and a default-deny seccomp allowlist. It creates the owner watcher and operation
threads only after both policies succeed. Inputs arrive on inherited pipes; the
worker has no database handle, source path, scratch directory or export grant.

| Resource or authority | Linux worker policy |
| --- | --- |
| Files and metadata | No pathname access; Landlock handles all ABI-3 filesystem rights and seccomp denies path syscalls. |
| Network and IPC sockets | Socket creation and socket syscalls are denied, including TCP, UDP and Unix sockets. |
| Processes | Fork, exec and process-form clone are denied. Only thread-form clone flags are admitted; clone3 returns ENOSYS so libc can use the inspected clone form. |
| Other owners | Process signalling is denied; tgkill is restricted to the worker's own thread group. RLIMIT_NPROC is not changed because it counts the shared host UID. |
| CPU and memory | One second of process CPU; RLIMIT_AS bounds initial virtual mappings plus 128 MiB. This is not an RSS ceiling. |
| Files and handles | Zero core dump size, 32 descriptors and 16384-byte file-size limit. No writable file grant is provided. |
| Threads and Runs | The common executor admits two node threads plus its owner watcher per worker. CPU/VM limits cover the whole process; this is not a kernel thread-count quota. The engine bounds active/queued Runs and enforces the two-second deadline. |
| Enforcement failure | `filesystem_policy_failed`, `syscall_policy_failed` or the corresponding resource-policy error terminates the worker without executing Core. The supervisor records failed work. |

The adversarial fixture verifies inherited denial inside a real thread, unchanged
host-UID process limits, refusal when the Landlock syscall is unavailable and
survival of an unrelated process. The common native matrix executes every eligible
Core operation and verifies retained outputs after restart. Worker crash,
cancellation, deadline, queue and owner-death tests use the common supervisor.
See [Linux verification commands](../development.md#validate-linux-workers).

Policy references: [Landlock](https://docs.kernel.org/userspace-api/landlock.html)
and [seccomp filtering](https://docs.kernel.org/userspace-api/seccomp_filter.html).

Run workers inherit no host environment. The supervisor supplies only a fixed
allocator setting: `MallocMaxMagazines=1` on macOS and `MALLOC_ARENA_MAX=1` on Linux.
These prevent allocator reservations from exhausting the same 128 MiB VM-growth
allowance on small parallel graphs. They grant no additional memory or authority.
See [Apple's magazine limit](https://github.com/apple-oss-distributions/libmalloc/blob/main/src/malloc.c)
and [glibc's arena limit](https://sourceware.org/glibc/manual/latest/html_node/Memory-Allocation-Tunables.html).

Electron preparation copies the engine executable into a distinct `play-worker`
helper and adds a distinct `play-terminal` helper. It records every engine, CLI, worker
and terminal digest. Startup verifies every required
binary before launching the engine; missing or changed helpers refuse startup.
The worker receives only its restricted invocation mode and private pipes.
Notebook/canvas execution and attached CLI commands share this worker path.
The interactive Linux terminal uses the same coordinator and scoped evidence.

Distribution packaging and the remaining runtime retirement are separate work in [#62](https://github.com/cubearc/play/issues/62).

The release target matrix is Windows x64, macOS x64/arm64 and Linux x64. Windows
still needs a proved filesystem/network boundary as well as resource and process
ownership; Job Objects alone are insufficient. These worker proofs advertise no
release-supported distribution matrix.

Run `npm run desktop:test --prefix frontend` for real engine/worker and adversarial
capability checks, and `npm run desktop:smoke --prefix frontend` for actual Electron
invocation, preload limits and rejection of another window using the same URL.
The development probe is not reachable through the application bridge. Record
exact OS/architecture/tool versions and observed results; a platform check skipped
on another OS supplies no validation evidence for that platform.

## Private data and Cloud handoff

Native private stores live in per-user application data, outside authorized source
roots. The engine has the SQLite/asset authority; workers receive minimum inputs
and explicit output grants, with only the fixed allocator environment above.
Credentials belong in the OS credential facility and privileged broker, never in
renderer state, worker environment, reusable definitions, command history or
diagnostics. If protected credential storage is unavailable, reject persistent
credential storage.

The following remote-view and callback requirements belong to the separate
[Cloud client milestone](https://github.com/cubearc/play/milestone/4). They do not
gate local release; local private storage and native isolation remain required.

Remote Challenge content must run in a separate unprivileged session/view without the
application preload, local resource grants or IPC handlers. Content URLs and
downloaded files never execute automatically. The proof rejects external views and
implements no Cloud callback.

The desktop Cloud callback must validate the exact scheme/host/path, one-use
state bound to the initiating desktop session, PKCE exchange and expiry. Reject
unexpected parameters, replay, arbitrary asset/file paths, command payloads and
redirect destinations. Opening a deep link alone never grants Workspace, export
or execution authority. Official submissions/publication enumerate the selected
revision and evidence; local Candidates/Solutions remain private and unverified
until a separate official record says otherwise.

The final application removes every superseded runtime listed in the
[retirement inventory](desktop-domain.md#interface-retirement-inventory).
The proof adds no old-store reader, model alias, schema converter or Docker/HTTP
execution fallback. The current application paths remain until their validated
replacements land; this development proof does not claim that retirement is done.

## Attached Cube CLI commands

**Run with Cube CLI** starts the bundled executable as a trusted control client.
Electron main supplies a fixed Workspace/Exploration/Stage and storage generation
through inherited private pipes. The closed preload accepts typed Asset, Pipeline
and Run intents; it exposes no executable path, shell text or arbitrary engine
command. Main constructs the CLI arguments and validates the request against that
exact intent before dispatching it. Native operation execution still belongs to
the engine's restricted worker. The control client is part of the application's
trusted code; it is not an operation sandbox or a native PTY.

The attached protocol is version 1 with Cube CLI 0.18.0. Each invocation exchanges
one context, one request, one reply and one result frame. A private random nonce
correlates the frames; it is never passed in argv or renderer data. Inherited
pipes provide the channel, not a public endpoint or environment token. Version,
context, command and result mismatches refuse the exchange. The host returns its
own engine result only after the client confirms that exact response and exits.

Frames use strict UTF-8 JSON and have a 262,144-byte ceiling including newline.
The CLI rejects duplicate keys, depth beyond 64 and node counts beyond 262,144;
these are aggregate transport budgets, not operation or Pipeline limits. Main
caps cumulative stdout at two frames and drains at most 16,384 stderr bytes
without forwarding private diagnostics. One CLI command may be active; its
5-second deadline kills and reaps the client. Closing the app reaps it before
closing the engine. A lost acknowledgment does not erase an accepted Run: retry
with the same caller and intent key to recover it from the coordinator.

| Intent | Real CLI arguments after `cube play` | Scope |
| --- | --- | --- |
| Inspect Asset | `asset show ID --revision N` | Exact revision of an Asset referenced by a Block in the attached Stage; image metadata and bounded file preview, not host paths |
| Inspect Pipeline | `pipeline show ID --revision N` | Exact reusable definition in the attached Workspace |
| Start Run | `run start --pipeline ID --revision N --target NODE_ID` | Frozen Trail request supplied by the parent; the CLI cannot retarget it |
| Inspect Run | `run status ID` | Owned Stage Run and retained evidence |
| Cancel Run | `run cancel ID` | Same coordinator cancellation; terminal records remain unchanged |

These commands refuse missing attached context and never fall back to a launcher,
filesystem search, shell or network service. External terminal discovery and
arbitrary shell execution are not provided by this channel.


## Interactive native PTY

On macOS, the Rust engine opens the real PTY, owns its dimensions and launches the
signed Terminal helper. The helper applies the native resource policy, then
executes the bundled `cube play session` in the same PID. The CLI inherits App
Sandbox and resource restrictions; it cannot read private/source files, connect
to a loopback server or spawn children. The native probe tests these restrictions
after the production exec boundary, including memory and CPU ceilings.

On Linux, the digest-verified `play-terminal` helper links the same pinned
`cube-cli::play` library used by the Cube executable and calls its `Session`
entry point. The helper installs Landlock and seccomp before the CLI reads any
private context. No copied command parser, shell or post-isolation exec is used;
no executable, shared-library directory or source-file grant is needed.

The Linux terminal keeps the worker's CPU, VM-growth and descriptor ceilings.
Its syscall policy additionally permits duplication of the inherited control
socket, peer inspection and destination-free reads/writes. Only `TCGETS` and
`TIOCGWINSZ` queries on stdin/stderr are allowed; resize remains parent-owned.
Path lookups, socket creation, external connections, fork/exec and signalling
other processes remain denied. Worker policies do not gain these terminal rights.
The same fixed allocator environment is applied before starting either helper.

Stdin and stderr use the PTY; stdout is an inherited private duplex Unix socket.
Terminal text is presentation, never an engine command or captured evidence.
The separate socket uses the same version-1 attached frames, with a fresh nonce
and selected Run intent key after every acknowledged command. Workspace,
Exploration, Stage and storage generation stay fixed. The engine compares a Run
request with the exact frozen selection and delegates only Asset/Pipeline reads
and Run start/status/cancel to its existing coordinator. No shell, database path,
PID, environment token or arbitrary process API crosses the renderer bridge.

`terminal.open` carries exact context, an explicit nullable Run selection, columns
and rows. Subsequent commands carry context and the session UUID. The CLI parses
ASCII command syntax; source content travels through typed Asset references.
`selected` is an alias for starting the frozen Pipeline request. Each completed
command receives a new intent key; repeated commands create separate Runs. A lost
reply must be reconciled in Run history before submitting again.

| Resource | Ceiling or policy |
| --- | --- |
| Concurrent sessions | One per engine |
| Session lifetime | 300 seconds; renderer read/write/resize lease expires after 10 seconds |
| Entered lines | 64, including help, blank and invalid commands |
| Input | 1,024 UTF-8 bytes per write; 1,024 ASCII bytes per line; 65,536 aggregate bytes |
| Output | 262,144 aggregate PTY bytes; eight queued 1,024-byte chunks with backpressure |
| Human reply display | 16,384 UTF-8 bytes plus truncation notice |
| Dimensions | 20–300 columns and 4–100 rows |
| Private frames | Existing 262,144-byte strict JSON ceiling and bounded two-frame queue |
| CLI resources | Existing helper CPU, VM-growth, descriptors, file-size and no-child limits |

The panel polls bounded output; it neither stores scrollback nor schedules nodes.
Only admitted Runs become evidence. Closing the terminal kills and reaps its CLI
without canceling an already admitted Run. Cancellation uses the coordinator and
waits for the operation worker to exit. Closing/switching Workspace drops the
session. A lost renderer lease closes it within ten seconds. A CLI reader thread
exits on control-channel EOF or failure even while waiting for terminal input;
a real engine SIGKILL test verifies the CLI disappears and unrelated processes
survive. Unsupported hosts reject session creation with `native_policy_unavailable`.

This proof does not establish external discovery, shell job control, grandchildren,
Windows PTYs or signed distribution support. The [terminal guide](../terminal.md)
owns the interactive workflow.

## Local Challenge import boundary

The trusted Electron frame requests a native chooser with only Workspace identity
and an action key. Main reads one regular file with a 128 KiB limit and sends
`import_challenge` to the engine. The renderer cannot provide package bytes or a
path. The engine parses the closed [package format](challenge-package.md), validates
content and commits the ordinary notebook entities and provenance together.
There is no archive extraction, external content fetch or automatic execution.
