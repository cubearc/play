# Desktop IPC and native execution

The desktop boundary connects a packaged React renderer to a Rust engine through
Electron's privileged main process. This page owns that boundary; the
[domain contract](desktop-domain.md) owns records and evidence. Delivery and
unresolved platform evidence remain in [the native contract issue](https://github.com/cubearc/play/issues/58).

## Implemented development proof

`npm run desktop:build --prefix frontend` builds `play-engine`, the packaged
renderer resources and the preload. `npm run desktop:proof --prefix frontend`
opens the actual Electron application. No web server, Docker or browser launcher
participates in this path. The [notebook preview](../development.md#explore-the-notebook-preview)
uses this boundary for managed notebooks. The engine persists Workspace content
and reasoning in [native storage](desktop-storage.md), and resolves exact owned
text Asset revisions for Core execution in a separate worker. Single-node
[PipelineRuns](desktop-runs.md) preserve accepted configuration, actual attempts
and the chosen private output retention. Credential storage is not implemented.

The proof is executable on macOS with a locally ad-hoc-signed App Sandbox helper.
Linux and Windows negotiate the protocol and expose Core adapter descriptions,
but reject execution with `native_policy_unavailable`. Neither has an
unrestricted or container fallback. This is not a release-supported desktop
distribution. The complete application and native terminal still require the
remaining shell, storage, worker and packaging deliveries.

The proof adapters invoke `cube-core`'s `cipher.rot13` and
`esolang.brainfuck.execute`. Their descriptions and versions come from actual Core
descriptors. Display names use Core's canonical name, such as **ROT13**, or its
human CLI spelling when no profile name exists. Stable IDs still identify
invocations; this is not a replacement operation catalog. The complete current
typed API conformance matrix remains required. Worker input is UTF-8 text and its bounded envelope holds `text` or `bytes`.
The supervisor maps successful output to Core's named ports and commits the Run.
The private unbound capability probe returns only its test envelope.

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

The bundled CLI discovers a per-Workspace native endpoint through a private
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
`cancelRun(context, runId)` as one typed object, and `dirty(boolean)`. Closed request shapes and UTF-8
limits are checked in preload before forwarding across IPC; main and Rust check
their own boundaries again. No raw `ipcRenderer`, arbitrary channel, path, Node,
filesystem, database or shell API crosses the bridge.

Packaged resources use an exact allowlist of `index.html`, `app.js`, `app.css` and
the bundled Codicons font `app.ttf`.
The protocol handler never converts a request pathname into a filesystem path.
Unknown names, hosts, methods, credentials, query parameters and fragments are
rejected. Encoded traversal cannot expand the resource allowlist. A restrictive
CSP denies network connections, frames, forms, inline scripts and object content;
font loading is restricted to the same packaged origin.
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
| `start_run` | Closed Run `request` | `run` containing the accepted or idempotently recovered record. |
| `cancel_run` | Exact `context`, `run_id` UUID | `run` after killing/reaping and recording cancellation, or the original terminal record. |
| `invoke` | `invocation`: `operation`, `operation_version`, `input` | Unbound native capability probe only; managed engine rejects `scoped_invocation_required`. |
| `cancel` | Session-local `run_id` | Unbound capability probe only; managed engine rejects `scoped_cancel_required`. |

Protocol **7** uses a 262144-byte frame ceiling, including bounded Base64 PNG
imports. The renderer has no literal invocation, old asset-execution alias or raw
image-import bridge.
Workspace intents cover list/create/open, create/open Exploration, create Stage,
add text, revise existing text/captions, reorder, duplicate, archive or restore Stage content Blocks,
save reasoning, list scoped Run summaries
and read Run details. Internal image import/read commands are Main-owned.
Every contextual request carries Workspace, Exploration and Stage UUIDs; the
engine resolves content and rejects mixed owners or absent exact revisions.
Stage responses expose one tagged active Block sequence, including reasoning,
plus explicit archived records. Trail views expose their derived `archived` state.
`reorder_blocks` pins the Stage revision and every Block UUID; see
[Block ordering](desktop-storage.md#block-ordering) for conflict and identity rules.
`duplicate_block` pins that structural revision plus the primary and optional
caption Asset IDs/revisions. [Copy provenance](desktop-storage.md#block-copies-and-provenance)
defines the independent identities, transaction and immutable origin record.
`archive_block` and `restore_block` use that same exact Block/source snapshot and
Stage revision. [Block lifecycle](desktop-storage.md#block-archive-and-restoration)
defines retention, restoration and execution admission.
The `revise_text` intent pins its Block/Asset IDs and expected revision. Run
requests require `source_asset_id` and `source_revision` as well as the selected
Pipeline revision; stale bindings fail before admission. Prior protocol shapes
are refused, without optional-field defaults or a compatibility command.
Managed `run` replies acknowledge durable admission immediately. Clients query
recorded progress/detail; the engine commits final state independently and never
sends unsolicited transport replies. The unbound `invoke`/`cancel` entry remains
only for native capability tests and is absent from the renderer bridge.

The first command must be `hello` within the handshake deadline. A mismatched
binary/protocol or malformed first frame closes the session. The nonce binds the
reply to this inherited pipe; it is not a substitute for authenticating external
CLI peers. There is no older-version negotiation or fallback interpreter.

`finished` has `status` of `completed`, `failed` or `cancelled`; only completed
responses contain an output. Rejections have a bounded machine-readable `code`.
Diagnostics do not echo input, paths or stderr. A busy engine rejects a new
invocation; the proof has one active worker and a bounded inbound command queue.
The renderer does not schedule operation dependencies.

Cancellation kills and waits for the owned worker before announcing its terminal
state. The private `started` frame includes the worker PID for supervisor
diagnostics; the preload never forwards it. A native test verifies that this PID
is gone while the engine remains alive after acknowledging cancellation.
EOF and malformed subsequent frames drop the supervisor and reap its
active child. The supervisor enforces the run wall deadline. Main rejects pending
calls on transport failure/timeout, closes the pipe, then kills an unresponsive
engine after a short shutdown grace. It never reports interrupted work as success.

## Lifecycle and recovery

Closing a window revokes that frame; the application and engine keep supervising
accepted Runs while alive. Activation opens another trusted window.
Unsaved reasoning prompts for keeping the editor open or discarding its draft.
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
| Crash cleanup | Session EOF and explicit cancel kill/reap the single child. | Prove engine crash, app crash, grandchildren and PTY lifetime on the native OS. |

macOS uses the supported App Sandbox entitlement on a signed helper bundle, not
`sandbox-exec` or private Seatbelt initialization. Its initial runtime/shared-cache
mappings are measured through the safe `libproc` API before setting the additional
VM allowance. Refusing the memory policy aborts the worker; no failed limit is
silently ignored. Core's own Brainfuck work/memory/output limits also apply.

The release target matrix is Windows x64, macOS x64/arm64 and Linux x64. Linux
requires a proved filesystem/network/process policy such as Landlock plus seccomp;
Windows requires an AppContainer/restricted-token boundary plus Job Object resource
and tree ownership. A Job Object alone or Unix file permissions alone is not a
filesystem/network sandbox. These mechanisms and their OS-version floors require
native validation; the proof advertises no release-supported matrix yet.

Run `npm run desktop:test --prefix frontend` for real engine/worker and adversarial
capability checks, and `npm run desktop:smoke --prefix frontend` for actual Electron
invocation, preload limits and rejection of another window using the same URL.
The development probe is not reachable through the application bridge. Record
exact OS/architecture/tool versions and observed results; a platform check skipped
on another OS supplies no validation evidence for that platform.

## Private data and Cloud handoff

Native private stores live in per-user application data, outside authorized source
roots. The engine has the SQLite/asset authority; workers receive minimum inputs
and explicit output grants, with a cleared environment. Credentials belong in the
OS credential facility and privileged broker, never in renderer state, worker
environment, reusable definitions, command history or diagnostics. If protected
credential storage is unavailable, reject persistent credential storage.

Remote Challenge content runs in a separate unprivileged session/view without the
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
