# Native desktop architecture

<a id="architecture"></a>
<a id="core-and-storage-boundaries"></a>

The implemented Electron preview loads a local React workbench, uses a narrow
preload to reach Electron main, and sends scoped commands over private pipes to
the Rust engine. It uses no browser server or Docker runtime in this path.
Algorithms, operation metadata and input/output contracts belong to `cube-core`.

## Runtime ownership

| Component | Responsibility |
| --- | --- |
| React renderer | VSCode Elements workbench, Stage notebook, Asset canvases and inline Run views |
| Typed preload | Closed application commands without Node, path or database authority |
| Electron main | Trusted-frame validation, native file chooser, app lifecycle and exact engine startup |
| Rust engine (`crates/play-engine`) | Ownership, private SQLite, revisions, admission and Run supervision |
| Bundled Cube CLI | Attached commands and macOS/Linux contextual PTY; private control channel, no domain database |
| Native worker | Bounded Core operation execution under the platform policy |

The preview has one trusted window and one active managed Workspace. The full
domain permits multiple independent Explorations in that Workspace. Multiwindow
attachment and external contextual CLI discovery are not implemented. The native
terminal uses a Rust-owned PTY and the same scoped Run coordinator.

## Domain and persistence

**Workspace → Exploration → Stage → ordered Blocks** organizes local work. An
image and caption share one Block while exposing separate Asset identities and
revisions. A Trail belongs to an Exploration and binds Assets to a Workspace
Pipeline definition. A PipelineRun freezes an invocation and its node evidence.

The engine owns private SQLite and an OS-held owner lock. Scoped commands check
Workspace/Exploration ownership before reading or writing. Reasoning, Asset and
Pipeline revisions are immutable; accepted graph Runs preserve their plan,
inputs, actual attempts and chosen output retention. Sources remain separate and
unchanged. [Native storage](reference/desktop-storage.md) owns exact format,
upgrade/refusal rules, quotas and recovery.

The [domain contract](reference/desktop-domain.md) defines the complete target
model. [Stage Candidates](solutions.md) select immutable Asset revisions or retained
Run outputs. Solution snapshots and official Cloud outcomes remain separate
deliveries; Run records alone do not imply either.

## Request boundary

Only the registered top frame at `cube-play://app/index.html` can use the bridge.
Renderer sandboxing and context isolation are enabled; Node integration, webviews
and arbitrary navigation are disabled. Another window loading the same URL does
not acquire authority. Main owns file selection and passes bounded content; the
renderer cannot submit arbitrary host paths.

The engine supervises real workers independently of React. It records acceptance
before dispatch and commits terminal evidence before reporting success. Worker
inputs are minimum scoped bytes, not database or credential access. The
[IPC contract](reference/desktop-runtime.md) owns exact frames, commands, sender
checks, resource accounting and adversarial validation.

## Native execution and delivery scope

The macOS proof uses a locally ad-hoc-signed App Sandbox helper with filesystem,
network, CPU/time, output and process controls. Typed graphs discover pure executors registered by their owning Core modules.
The host negotiates representations and retention policy; it contains no operation-ID
execution dispatcher. See [Pipeline execution capabilities](reference/desktop-pipelines.md#execution-capabilities).
Independent branches execute in groups of at most two within the same sandboxed
Run worker, sharing its CPU, VM, wall-time and output limits. Dependency-aware
attempt admission and deterministic result collection preserve join evidence.
The coordinator owns up to two Run workers and four queued tasks, rotating fairly
between Explorations and admitting only one active worker per Exploration.
Linux x64/ARM64 Electron workers use Landlock and seccomp with the same Core
executor and supervisor. Native-host checks validate their boundary; Electron
journeys exercise actual dispatch and retained evidence. Windows worker enforcement
remains incomplete; no unrestricted fallback is available.

External CLI attachment, Windows PTYs,
signed packages and verified updates remain incomplete. Local build digests and
ad-hoc signing are development checks, not publisher authentication. See
[contributor verification](reference/validation.md) for actual checks.

The [local Electron milestone](https://github.com/cubearc/play/milestone/3)
includes implemented [canonical local Challenge package import](challenges.md)
and private Stage Candidates. Solution snapshots remain pending.
Cloud is an [independent project](https://github.com/cubearc/play-platform/issues/10),
with separate [service](https://github.com/cubearc/play-platform/milestone/1) and
[desktop integration](https://github.com/cubearc/play/milestone/4) milestones.
Authentication, online Challenge cloning, progressive authorization, submissions,
publication and official rankings/rewards do not gate local completion. Cloud
integration will consume the canonical local contracts.

## Repository transition

The source tree still contains `play-local`, `play-launcher`, `play-runner`, the
served web frontend and Docker checks. They are outside the Electron invocation
path. [#56](https://github.com/cubearc/play/issues/56) owns their required removal;
this documentation update does not establish retirement. Existing tests remain
required until reviewed native replacements land. There is no native fallback
into those runtimes and no old product-store converter.
