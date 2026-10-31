# Desktop domain contract

Use this contract when implementing the Electron investigation IDE. It defines
entity ownership, evidence identity and the required retirement of superseded
interfaces. The [architecture](../architecture.md) describes the runtime currently
implemented. Delivery status and scope belong to the
[local Electron refactor milestone](https://github.com/cubearc/play/milestone/3), rather
than a second repository backlog.

The [Cloud project](https://github.com/cubearc/play-platform/issues/10) separately
owns remote services and their desktop integration. Local Challenge package
import, Candidates and Solutions remain part of the local milestone. Cloud
origins and official outcome boundaries below define future integration contracts;
they do not require a Cloud account or service to complete local Play.

## Canonical vocabulary

| Term | Meaning |
| --- | --- |
| Workspace | A local identity with authorized source roots, private storage and a native execution boundary. |
| Exploration | An independent investigation inside a Workspace. It owns its local notebook, hypotheses, evidence and progress. |
| Stage | An ordered local part of an Exploration, optionally derived from a published Challenge stage. |
| Block | An ordered content item in a Stage: text, image with an optional caption, file, inert page, annotation or retained result. |
| Asset | A logical, revisioned identity for content that can be inspected or used as an operation input. |
| Trail | An Exploration's hypothesis and investigation history, with a selected Pipeline revision and explicit input bindings. |
| Pipeline | A reusable, versioned graph of typed operation nodes and declared input slots. |
| PipelineRun | One accepted invocation of an exact Pipeline revision with resolved inputs, node attempts and reuse decisions. |
| Node attempt | One actual execution of a node within a Run, including its status and retained evidence. |
| Candidate | A private, revisioned answer selection for a local Stage. |
| Solution | An immutable revision of a chosen result, explanation and contributing evidence path. |
| Challenge edition | An immutable published activity version identified by the remote service. |

Exploration names the investigation container throughout Rust, TypeScript,
storage and commands. Trail/Pipeline own hypothesis and graph responsibilities;
`Investigation` is excluded from the model. The superseded implementation is
removed, as recorded in the retirement inventory below. The native cutover does
not translate old records or expose those names as aliases.

## Ownership and cardinality

Every local command carries Workspace and Exploration context when accessing
Exploration-owned data. Validate ownership in the Rust engine before resolving
references or changing records. A renderer selection, known UUID, content digest
or filesystem path does not grant authority.

| Data family | Owner and relationships | Reference rule |
| --- | --- | --- |
| Workspace grants and private stores | One Workspace; many Explorations share its authorized execution/storage boundary. | Source grants are distinct from writable private storage and exports. |
| Exploration | Exactly one Workspace. | Local work is never assigned through a default singleton or global active Challenge. |
| Stage | Exactly one Exploration; zero or more per Exploration, in explicit order. | Optional origin identifies the source edition/stage; it does not replace the local Stage ID. |
| Block | Exactly one Stage; zero or more in explicit order. | Content references resolve within the Stage's Exploration. An image Block can include its caption; they render together while retaining independent Asset identities and revisions. |
| Asset and its revisions | Exactly one Exploration, with an optional originating Block or node output. | Resolve an exact revision and representation through authorized storage. |
| Imported files and retained output bundles | Private bytes belong to the Workspace store; logical Asset/result records belong to one Exploration. | Shared physical storage does not make another Exploration's records visible. |
| Trail | Exactly one Exploration; optionally one local Stage. | Its chosen Pipeline belongs to the same Workspace. Trail bindings/evidence stay in the owning Exploration. |
| Pipeline and configuration revisions | Exactly one Workspace; many Trails can select one revision. | Reusable definitions use declared inputs, with no embedded Exploration-specific Asset bindings or secrets. |
| PipelineRun | Exactly one Exploration and Pipeline revision; optionally one Trail. | Record the actual target set, input revisions, caller and reuse policy. Ad hoc operations use the same single-node Run contract. |
| Node attempt and output ports | Exactly one Run and node in its pinned Pipeline revision. | A reused output points to its original attempt; it is not another successful execution. |
| Note | Exactly one Exploration; optionally targets its Stage, Block, Asset, Trail, Run, attempt or named output. | Annotation Blocks use the same note identity/revision rules instead of an independent notes store. |
| Candidate revision | Exactly one local Stage and its Exploration. | Freeze typed output/Asset revision, field selection and explicit conversion. |
| Solution revision | Exactly one Exploration; optional local Stage/Candidate association. | Freeze only selected contributing evidence; working alternatives remain in their Trails. |
| Local progress | Exactly one Exploration/local Stage pair. | Two Explorations of one Challenge have independent local status. |
| Saved navigation and drafts | Explicit Workspace/window and Exploration context. | Cache keys, pending writes and events include that context; switching views cannot retarget an in-flight write. |
| Engine, terminal and worker sessions | Workspace-owned process/session identity with explicit Exploration/Run context for investigation actions. | Session IDs are not investigation IDs; no automatic full command/scrollback recording. |
| Remote origins and participation cache | Scoped by service, Challenge, edition and participant, with explicit local associations. | Shared official participation can expose the same authorized official status without merging local work. |
| Submission, evaluation and publication receipts | Service-owned records cached against the chosen local revision and participation. | Local edits cannot mutate an already submitted or published revision. |
| Official completion, ranking and payout records | Service-owned, edition/participant-scoped authorized data. | A local flag, replay report or content hash cannot create official acceptance or payment. |

### Explicit reuse

A reusable Pipeline contains operation IDs/versions, named ports, ordinary
non-sensitive parameters/constants, layout and input declarations. A Trail or
Run binds those declarations to its own Assets. Editing another Trail's binding
cannot alter the definition or an earlier invocation.

To reuse content across Explorations, require an explicit user action that
creates a destination-owned Asset/reference with source provenance and applicable
file grants. Do not silently attach the source Exploration's notes, private
bindings, Candidates, Solutions or progress. Read-only physical bytes may be
deduplicated, but ownership checks still apply to every logical reference.

## Identity, revisions and origins

Local entities have engine-generated opaque UUID identities. Names, ordering,
paths and content hashes are not primary keys. An edit retains the entity ID and
creates the appropriate revision; an explicit copy has a new identity and source
lineage. A file move can make content unavailable without changing a saved
reference or inventing a new official origin.

Keep these versions distinct:

- Storage generation and schema version identify the accepted local store.
- Command and content-format versions identify their serialization contracts.
- Entity/configuration revisions identify edits to a particular local entity.
- Core operation/schema/adapter versions identify executable contracts.
- Challenge edition/content revisions identify immutable published material.

A remote origin includes the trusted service identity, Challenge ID, edition ID
and remote Stage/content ID as applicable. Local package identity is explicitly
local. Neither a manifest's filesystem path/hash nor `format_version` establishes
official identity, participation or correctness.

Writes use explicit expected revisions. Stale edits return a conflict without
last-writer-wins replacement. Serialize/coalesce autosaves within their original
context; a response or event from Exploration A cannot update Exploration B.
Idempotency keys identify accepted actions in their owner/command scope and must
not turn a retry with different content into the same successful action.

## Evidence, privacy and retention

Configuration revisions, Asset content revisions, finished node attempts and
Solution/Candidate revisions are immutable once committed. Mutable current
pointers and local status use optimistic revisions. A retry creates another Run
or attempt; it never edits a completed execution into a new success.

A Run records its exact Pipeline revision, resolved Asset revisions, actual
named outputs and selected dependency/reuse edges. Reuse requires matching input
identity, contracts and available authorized content. Changing a file or caption
cannot silently change previous evidence. Missing or changed sources, missing
bundles and unavailable/redacted inputs produce explicit replay gaps.

Algorithms, conversions and operation metadata belong to Core. Asset references
identify semantic type and representation; selecting a field or converting
text/bytes/numbers is explicit. Keep every installed current operation covered by
real contract/fixture conformance, including direct-only operations.

Honor Core's sensitive-input/parameter/output policy. Per-run secrets do not
enter reusable definitions, persisted evidence or logs; do not retain prohibited
fingerprints. Private output retention is an explicit choice. User-authored notes
and explanations remain private local content until a chosen submission or
publication enumerates what will leave the machine.

Archive referenced entities rather than creating dangling evidence. Hard deletion
requires an unreferenced object and explicit authorization; shared bytes can be
reclaimed only after all live references permit it. Quota exhaustion rejects new
work visibly and preserves previous records. Bound both per-Exploration and
aggregate Workspace resources. Closing, upgrading or uninstalling the app never
implicitly deletes sources or private evidence.

## Native runtime boundary

Electron owns trusted application windows, native dialogs/open intents, lifecycle
and the narrow preload/main bridge. Keep renderer sandboxing and context
isolation enabled, with Node integration disabled. Validate the sending
window/frame and command context; do not expose arbitrary IPC channels, paths,
shell commands or SQLite handles.

The bundled Rust engine owns authorization, domain storage and Run orchestration.
UI and bundled Cube CLI use one versioned command contract over protected native
IPC. Scoped app-resource reads and exports resolve engine-authorized references;
they do not expose a general filesystem or a compatibility localhost API.

The terminal executes the real Cube CLI under that authorized context. Its
command protocol does not add shell expansion, pipes/redirection, arbitrary
executable paths or automatic command recording.

Native workers and PTYs execute Core and the real CLI outside Electron's UI/main
threads. Renderer sandboxing does not restrict those binaries. Each advertised
OS must prove its source/output grants, worker network policy, process-tree
cleanup and CPU/memory/time/process/output bounds. Unsupported enforcement fails
explicitly, without a container or unrestricted fallback. Exact IPC layouts,
limits and supported OS/architecture mechanisms are owned by the
[native execution contract](desktop-runtime.md).

In the separate Cloud integration, remote Challenge pages/forms must run in a
separate unprivileged view/session without
the application preload or local command authority. Bind authentication callbacks
to their desktop session. Cloud/update credentials remain in privileged
OS-protected storage and never enter remote content, workers or command logs.
Platform deployment technology does not become a local Play prerequisite.

## Fresh-store and unsupported-store cutover

The desktop release is a breaking generation cutover. There is one accepted
storage/content/command generation and no shipped old-store reader, migration
chain, manifest converter, model alias or alternate Docker runtime.

1. Authorize a source root and select private native storage separately. Opening
   a source directory does not authorize writes there or adoption of an adjacent
   database. Existing Docker registries/volumes are not consulted automatically.
2. Inspect any selected existing store's identity and generation read-only,
   before writable connection setup, journal changes, migrations, reconciliation
   or entity queries. Unknown, corrupt, older and newer unsupported generations
   fail with an actionable unsupported-store error.
3. On rejection, leave the database, WAL/SHM, bundles, registry, permissions and
   source bytes unchanged. Do not rename/reset the store, try an old schema, run
   Docker, import old volumes or create a partial replacement in that location.
4. Offer an explicit fresh private-store action. Refuse occupied/incompatible
   destinations; obtain the owner lock, create a new Workspace identity and
   initialize the canonical baseline atomically. Never reuse foreign owner IDs.
5. Reopening a supported store validates its identity, relationships and exact
   revisions. Recovery records genuinely unfinished Runs as interrupted and
   preserves completed evidence; it does not invent missing attempts or answers.
6. Keep prior stores/volumes as external archival material under the user's
   control. Recovery with an archived compatible application is external to the
   desktop runtime. Automatic conversion/deletion is not a delivery requirement.

The [storage implementation](https://github.com/cubearc/play/issues/35) assigns the
concrete generation marker, baseline and rejection fixtures. Only the current
baseline ships. Updates check compatibility before any
data-changing handover.

## Interface retirement inventory

These mappings define the final architecture and observable removal checks.
Linked issues own implementation/status; this page does not duplicate their
backlog. Remove a superseded path in the slice that replaces it, then audit the
complete tree and distributable under
[canonical architecture acceptance](https://github.com/cubearc/play/issues/56).

The source entries below are retired from the current tree. Their canonical
replacements and remaining capability/platform acceptance stay owned by the linked
issues; source deletion does not establish Windows enforcement or signed updates.

### Domain, storage and content

| Superseded surface and source | Canonical replacement | Delivery owner | Observable check |
| --- | --- | --- | --- |
| `migrations/0001_investigations.sql` through `0007_solutions.sql`; `persistence/mod.rs` singleton/`investigation_id` startup | Native generation baseline and explicit Exploration ownership | [Storage](https://github.com/cubearc/play/issues/35) | Old migration chain, singleton queries and old-store fallback are absent; rejected stores remain byte-identical. |
| `persistence/http.rs`, `records.rs`, `library.rs`; `/investigation` commands and unfiltered notes/library/progress | Explicit scoped engine repositories/commands | [Ownership](https://github.com/cubearc/play/issues/36) | Two Explorations cannot list or mutate each other's records; no ambient investigation API remains. |
| `InvestigationWorkspace.tsx`, `investigation-api.ts`, `WorkspacePanels.tsx`; file-only saved selection | Exploration navigation and typed Stage/Block/Asset/Trail/Run context | [Desktop navigation](https://github.com/cubearc/play/issues/37) | Context switches and delayed writes restore the correct owner; old shell/DTOs are absent. |
| `persistence/challenges.rs` global `active_challenge`; progress/candidate keys based only on Challenge/Stage | Local Stage identity and Exploration-scoped progress/Candidates | [Stages](https://github.com/cubearc/play/issues/38), [evidence](https://github.com/cubearc/play/issues/50) | Two imports of one Challenge have independent local selections/progress; no global active-Challenge query remains. |
| `challenges.rs` format-1 `instructions/steps/hints/artifacts`; `ChallengePanel.tsx`, path-derived import key | Canonical Stage/Block package and explicit local/remote origins | [Blocks](https://github.com/cubearc/play/issues/40), [import](https://github.com/cubearc/play/issues/41) | Superseded manifests are rejected without partial import; no parser/converter for them ships. |
| `persistence/explorations/model.rs` graph-as-Exploration `Definition`, configurations/executions and `exploration-api.ts` DTOs | Trail identity, reusable Pipeline revisions and whole Runs | [Pipelines](https://github.com/cubearc/play/issues/42), [Runs](https://github.com/cubearc/play/issues/44) | Only the container uses the Exploration name; graph/node evidence has canonical ownership. |
| Graph `Source` file/literal/artifact projections and filename-only note targets | Revisioned Assets, declared inputs and named typed ports | [Assets](https://github.com/cubearc/play/issues/39), [composition](https://github.com/cubearc/play/issues/49) | Caption edits preserve old input evidence; no legacy source-variant interpreter remains. |
| `persistence/candidates.rs`, `solutions.rs`; old JSON snapshots and `solution-api.ts` associations | Canonical Stage Candidate and immutable Solution revisions | [Local evidence](https://github.com/cubearc/play/issues/50) | No old snapshot reader remains; only selected contributing evidence enters a Solution. |

### Execution and compatibility

| Superseded surface and source | Canonical replacement | Delivery owner | Observable check |
| --- | --- | --- | --- |
| `lab/mod.rs`, `execute.rs`, `types.rs`, `worker.rs` dual request branch; single-input `/lab/execute`, v1 discovery and `frontend/src/lab-api.ts` producers | One typed engine invocation/result contract | [Typed producers](https://github.com/cubearc/play/issues/43) | Visual/quick tools and graph nodes share named outputs and Run evidence; no v1 dispatcher/DTO ships. |
| `lab/storage.rs`, `typed.rs`, `replay.rs`; dual result metadata, legacy conversion guard/`legacy_history`, unassigned history | Canonical bundles, scoped Assets and honest replay gaps | [Typed producers](https://github.com/cubearc/play/issues/43), [evidence](https://github.com/cubearc/play/issues/50) | No legacy bundle reader, conversion route or inferred old history remains; current actual-output fixtures still pass. |
| `lab/domains/puzzle_codes.rs::Legacy` transport helper and superseded adapter aliases | Exact installed Core named-input schemas | [Typed adapters](https://github.com/cubearc/play/issues/43) | Obsolete helper/aliases are removed or renamed to the actual contract; current book/keyboard operations retain full conformance. |
| `ExplorationLab.tsx::run`, recursive ancestor/reuse decisions and per-step frontend calls | Rust engine dependency planning and one Run request | [Coordinator](https://github.com/cubearc/play/issues/45), [Flow/Steps](https://github.com/cubearc/play/issues/47) | Renderer code does not schedule ancestors, choose reusable attempts or sequence execution requests. |
| `lab/mod.rs` single `Active` slot and `Semaphore::new(1)` orchestration | Bounded native worker registry/queue and per-Run state | [Concurrency](https://github.com/cubearc/play/issues/46) | Independent branches overlap within tested bounds; cancelling one Run leaves unrelated workers alive. |
| `lab/projection.rs` string-only UI composition and copied literal inputs | Explicit bounded Core selection/conversion/concatenation/numeric operations | [Composition](https://github.com/cubearc/play/issues/49) | No Play expression algorithm or silent coercion; real Core/Play fixtures cover every current operation. |
| `terminal.rs`, `terminal-api.ts`, container capture import transport and `/terminal/ws` | Native PTY/CLI session plus authenticated contextual engine bridge | [Native terminal](https://github.com/cubearc/play/issues/48), [CLI](https://github.com/cubearc/cube/issues/492) | Real bundled CLI uses the same context/Run contract; no WebSocket transport or terminal-text scraping remains. |

### Desktop host, transport and distribution

| Superseded surface and source | Canonical replacement | Delivery owner | Observable check |
| --- | --- | --- | --- |
| `lib.rs` HTTP/static routes; `security.rs`, `security/read_grants.rs`, `config.rs` browser startup fragment/session, cookie expiry and URL read grants; frontend `api.ts`/`capability.ts` | Trusted Electron frames, narrow typed IPC and scoped app resources | [Native contract](https://github.com/cubearc/play/issues/58), [shell](https://github.com/cubearc/play/issues/59) | Forged senders/resources fail; no local REST/static server, old cookie/capability bridge or compatibility URL path ships. |
| `play-launcher` host/server/picker/control, `LauncherHome.tsx`, `launcher-main.tsx`, `launcher-api.ts`, chooser-origin handoff | Electron dialogs/open intents, native workspace registry and app lifecycle | [Shell](https://github.com/cubearc/play/issues/59) | App/CLI open intents work without a browser chooser; no host launcher service or old client remains. |
| `play-launcher/src/docker.rs`, `runtime.rs`, `runner.rs`, Docker ownership labels/image/runtime aliases and named-volume registry | Bundled compatible app/engine/CLI and native private storage | [Packaging](https://github.com/cubearc/play/issues/63) | Clean-host install/open/restart needs no Docker/WSL; no container discovery, build, mount or compatibility launcher code remains. |
| `play-runner` Linux-only entry point, container Unix socket and Docker network/cgroup assumptions | Native per-OS workers/PTYs with enforced limits | [Windows](https://github.com/cubearc/play/issues/60), [macOS](https://github.com/cubearc/play/issues/61), [Linux](https://github.com/cubearc/play/issues/62) | Each native OS proves file/network/resource denial and owned-tree cleanup; no Linux VM/container fallback remains. |
| Local `Dockerfile`, `compose.yaml`, `.dockerignore`, Docker fixtures, `tests/runtime_smoke.py` container paths and CI Docker jobs | Native package/IPC/worker/PTY smoke and equivalent security/conformance coverage | [Retirement](https://github.com/cubearc/play/issues/56), [validation](https://github.com/cubearc/play/issues/57) | Local product/build requirements and current contributor docs have one native path; existing quality gates remain enforced. |
| Container-image upgrade/recreation and old runtime version switches | Signed app/engine/CLI updates and native recovery | [Updates](https://github.com/cubearc/play/issues/64) | Actual OS update paths preserve local data and reject incompatible/corrupt artifacts without invoking old runtimes. |

Current Core operation versions and algorithms remain governed by their canonical
contracts. A historical cipher name or the word `Legacy` alone does not establish
a compatibility path. Review actual reachability, schema, producers and consumers;
word counts cannot prove removal or justify dropping a current operation.

## Observable contract scenarios

Use these outcomes when implementing the linked behavior tests:

- A and B in one Workspace import the same Challenge. Distinct notes, local Stage
  progress, Trails, Candidates and Solutions survive app/engine restart.
- A request combines A's Exploration ID with B's Stage/Asset/Run. It fails before
  reading or writing B's content; a known ID or digest does not bypass ownership.
- Two Trails bind one Pipeline revision to different local Assets. A caption edit
  creates a revision, and old Run/Solution evidence still identifies its original
  input. Missing original bytes remain an explicit replay gap.
- A stale autosave/event arrives after switching Explorations. It retains its
  original context and cannot overwrite the visible Exploration.
- A node output is reused. The new Run references the original authorized
  attempt, rather than fabricating another execution; private policy still holds.
- A referenced Block, Asset or Pipeline revision is archived. Existing selected
  evidence remains resolvable; quota/deletion cannot silently erase it.
- An unsupported store is opened. Database/WAL/SHM/bundles/source hashes and
  permissions stay unchanged, with no migration, Docker invocation or reset.
- Untrusted renderer/remote-page/CLI context requests privileged work. The
  validated native boundary rejects it without extra filesystem/network grants.
- Local completion or replay succeeds. Official submission/evaluation/publication
  remain separate chosen-revision service actions, with no implicit upload.

The domain contract does not itself establish a native runtime test pass. Storage
fixtures, real per-OS execution evidence and final artifact/portal verification
belong to their linked implementation acceptance.
