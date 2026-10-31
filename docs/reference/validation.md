# Contributor verification

<a id="coverage"></a>
<a id="integrated-mvp-verification"></a>

Use the [native development workflow](../development.md) for Electron. The
commands below distinguish native proof from repository checks still required
while the served-web/Runner/container implementation remains in the tree.
Documentation-only delivery does not remove those production paths or CI gates.

## Native engine and Electron

From the Play root with the pinned Rust/Node toolchains and trusted Cube beside it:

```sh
npm ci --prefix frontend
npm run desktop:build --prefix frontend
cargo test --locked -p play-engine
cargo clippy --locked -p play-engine --all-targets -- -D warnings
npm run desktop:test --prefix frontend
npm run desktop:smoke --prefix frontend
```

The engine suite exercises real SQLite, immutable revisions, ownership, accepted
Runs and interruption recovery. Desktop tests use real pipes/workers and reject
forged sender/frame authority. The Electron smoke opens the actual renderer and
preload, imports a real PNG, decodes its caption, retains history, saves separate
Explorations and reopens the private profile. A second journey edits text/captions, preserves
earlier input results, tests conflict and unsaved-draft handling, and reopens the
new revisions. Native IPC also edits an input while a real worker runs, then
rejects stale dispatch. These checks preserve source bytes and reject
cross-Exploration access. A controlled native chooser fixture is not a Cloud mock
or a general file-browser implementation.

Worker lifetime checks run the signed production worker with its input pipe still
open and compare the actual ROT13 result. The signed supervision fixture uses the
production task reader and owner guard while holding a canonical node-start event;
EOF and additional input must stop it without a result. A real engine `SIGKILL`
test observes its child's PID disappear, keeps an unrelated process alive and
reopens the Run as interrupted with no fabricated output. The fixture has no
separate parent-death cleanup. Malformed/oversized task frames are also refused.
The multi-Run fixture proves two workers coexist, a worker crash preserves its
sibling, and owner death stops both while leaving an unrelated process alive.
Recovery distinguishes active attempts from queued Runs without attempts. The PTY checks below cover
the separate contextual CLI lifetime; grandchildren remain unsupported.

Ordering and duplication journeys verify completed keyboard focus, concurrent
revision conflicts, independent copied Assets and Runs, pinned source labels,
image/caption grouping and restart. Storage tests upgrade native schemas 1/2,
preserve immutable lineage and roll back a full copy if caption storage fails.
Archive checks preserve active worker supervision and history, refuse new
execution and edits, restore original IDs, exercise capacity/isolation, and verify
schema 3 upgrades plus database deletion/Note-state guards.
File checks verify exact imported bytes, bounded UTF-8/hex/empty previews,
regular-file admission, inert HTML, cancellation, exact chooser authority,
independent copies, ordering, archive/restore, Exploration isolation and restart.
Schema-4 fixtures preserve archived content, source lineage and completed Runs
before accepting file imports. A second same-URL window cannot invoke the chooser.
Result checks promote exact retained Run/attempt/port outputs, reject unfinished,
unretained, foreign and stale requests, preserve unique immutable origins and
database revision guards, reuse text through a new real Run, and save real
Brainfuck bytes as Files. The Electron journey follows the original Run after
later edits/dispatch, preserves reasoning drafts and result identities through
archive/restore and restart, and keeps a second Exploration independent.

Annotation checks verify independent Note content/revisions, copy lineage,
conflicts, scoped snapshots, byte limits, retained/restore capacity, immutable
identity and protected reasoning. A schema-6 fixture retains saved results, Runs
and reasoning before accepting annotations. The Electron journey saves only the
focused Note, preserves other drafts, rejects silent concurrent overwrites,
discards to the latest saved version, keeps markup inert, copies/orders/archives/
restores exact identities, isolates Explorations and reopens the private profile.

The complete notebook journey combines inert Page source, grouped image/caption,
File, annotation and Result Blocks. It uses keyboard creation/edit/copy/order/
archive/restore, selects compatible image and caption operations, checks exact
image Asset binding and RGB bytes, switches Explorations and reopens all saved
identities. Hostile page scripts/forms/embeds stay text. axe-core runs in the real
Electron document with WCAG 2 A/AA, 2.1 AA and 2.2 AA rules for notebook and image canvas;
no rules are disabled. Tests cover tab arrow/Home/End navigation, readable
contrast and keyboard access to the image viewport. Native IPC also saves actual
pixel bytes as a File result and rejects archived-image dispatch; schema-7
fixtures preserve Notes and bind prior image Trails without losing evidence.

Structured-composition journeys connect real image dimensions and caption integers,
exercise field selection and explicit text conversion, reject invalid literals,
missing fields, null-to-text and integer overflow, and preserve exact successful
and failed Runs through restart. Keyboard/ARIA checks cover disabled controls,
parameter fields and selectable file previews. Retention checks toggle enabled
checkboxes by keyboard, then verify disabled state and an unchanged checked value
when Space reaches an archived control. These checks do not establish
complete WCAG conformance.

Registry checks compare the native eligible set with Core registrations and execute
all registered operations against their canonical known-answer vectors. The native
IPC matrix discovers its cases without a separate operation list, imports image
vectors as scoped Assets, executes every eligible registration in the real isolated
worker and verifies its reviewed contract digest and all named outputs after restart.
Each case has an independent private store, so Workspace quotas do not limit matrix
coverage. Only canonical parameter defaults adopt lower host defaults; the report
records both configurations. It does not claim native execution for unregistered
Core operations or unsupported platforms. CI retains `native-operation-conformance.json`
for each tested macOS and Linux worker architecture. A real
Electron journey uses generated traversal/optional controls, refuses an invalid
array draft, connects the byte port through UTF-8 and ROT13, executes a separately
registered text operation, retains both image-export ports and reopens exact Run
evidence. Core schemas and the full API conformance matrix remain independent checks.

CI runs the Electron proof on macOS ARM64/Intel and Ubuntu 24.04 x64/ARM64. The
macOS worker is locally ad-hoc signed and sandboxed. Linux jobs retain the native
worker boundary, Core matrix and workspace/supervisor checks, then exercise the
actual notebook/canvas, attached CLI and retained evidence through Electron.
The interactive terminal journey executes on both platforms.
Playwright launches with Chromium sandboxing enabled; the Linux authority check
verifies renderer seccomp and no-new-privileges through `/proc`. Integrity tests
refuse altered/missing prepared binaries and a missing worker digest. See
[Linux verification](../development.md#validate-linux-workers).
Windows worker and PTY enforcement remain unproved; ordinary Rust/protocol
tests do not establish those capabilities. None of
these commands establishes publisher signing, notarization, signed updates or a
complete packaged Windows/macOS/Linux release.

## Quality checks

Build the remaining launcher assets before whole-workspace Cargo checks. From the
root:

```sh
npm ci --prefix frontend
npm run build --prefix frontend
cargo fmt --all -- --check
rustfmt --check --edition 2024 crates/play-launcher/tests/support/docker_fixture.rs crates/play-launcher/tests/support/folder_picker.rs crates/play-runner/tests/support/resource_probe.rs
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --locked --manifest-path ../cube/Cargo.toml -p cube-cli
cargo test --workspace --all-features --locked
npm run check --prefix frontend
npm run build --prefix frontend
npm run test:coverage --prefix frontend
cargo build --workspace --locked
```

The native Rust CI matrix sets `RUST_TEST_THREADS=1` to avoid independent SQLite
fixtures competing for initialization deadlines on shared hosts. On Unix use
`export RUST_TEST_THREADS=1`; on PowerShell use
`$env:RUST_TEST_THREADS = '1'`. Concurrent-request tests still exercise concurrency.
Do not raise production deadlines or lower gates to compensate for CI contention.
The native Node test command also runs independent test files one at a time.
Worker deadlines and process CPU/VM ceilings stay unchanged; the tests for
parallel nodes still synchronize two real execution threads inside one Run.

## Coverage and Core conformance

```sh
cargo install cargo-llvm-cov --version 0.9.1 --locked
cargo llvm-cov --workspace --all-features --locked --fail-under-lines 80 --ignore-filename-regex '/tests/'
cargo llvm-cov report --lcov --output-path lcov.info --ignore-filename-regex '/tests/'
```

Rust's line gate is 80% of application source, including startup and excluding
test sources. Frontend statements, branches, functions and lines each have an
80% gate. Its Vitest percentage covers the configured web frontend sources, not
the entire desktop renderer; real Electron checks provide separate evidence.
Record actual coverage and retain LCOV, rather than inferring it from assertions.

The typed API matrix must execute every installed Core operation ID against its
real fixture/version/schema and validate named outputs. It writes
`target/play-operation-conformance.json`. Reject missing/duplicate IDs and drift;
there is no count threshold or handwritten substitute catalog. Integers, decimal
strings and bytes remain exact; floating-point analyses use relative tolerance
1e-12. The served Lab accepts `file.info.inspect` version 2 with either an
authorized workspace-file binding or literal byte data. Selected files retain
the scoped filesystem checks; byte data reports its actual size and signature
through Core and leaves filesystem-only facts null. Literal paths and artifact
references grant no filesystem access. The matrix uses exact byte-artifact
answers, while separate tests validate selected-file metadata and authority.
Regenerate canonical contracts in Cube with
`cargo run --locked -p cube-core --example operation_contracts`.

## Remaining web and container checks

These validate production paths scheduled for removal in
[#56](https://github.com/cubearc/play/issues/56). They are contributor requirements,
not installation steps or a fallback execution policy for Electron.

For the remaining real browser tests, run from `frontend/`:

```sh
npx playwright install chromium
npm run test:e2e
```

On Linux, use `npx playwright install --with-deps chromium` when system libraries
are needed. The harness serves the production frontend through a real Rust server
on loopback port 17331, with fixture-owned private directories. It verifies
session/Host/Origin authority, Core-backed previews, inert hostile content, byte
ranges, imports, persistence, conflicting tabs and responsive layouts. Browser
traces remain disabled to avoid recording session capabilities. Container-only
PTY cases are separate from this host browser suite.

For real Linux container/Runner behavior, from the root:

```sh
python3 tests/runtime_smoke.py
docker build --target runner-tests --build-context cube_engine=../cube .
docker build --target lab-tests --build-context cube_engine=../cube .
```

These checks require their own local Linux container environment. The runtime
suite owns unique projects/ports/volumes and checks actual startup, CLI/PTY,
filesystem/network/resource boundaries, source hashes and recreation/recovery.
Only fixture-owned resources may be removed. Use `CUBE_PLAY_CUBE` and
`CUBE_PLAY_LAUNCHER` for exact trusted executables and `CUBE_RUNNER_CUBE` for the
real Linux CLI when needed. Keep Runner test binaries on the Linux filesystem.
Controlled Docker CLI fixtures test errors; they do not replace the real suite.
WSL container results and native Windows checks are different evidence.

For the Windows-browser/WSL chooser fixture, only
`CUBE_PLAY_CHOOSER_SELECTION` points to its Windows-accessible selection file;
`CUBE_PLAY_CHOOSER_PARENT` and `CUBE_PLAY_CHOOSER_SOURCE` stay Linux paths. This
fixture override does not authorize production UNC mounts. On Intel macOS, the
web API tests may require ad-hoc signing of the exact rebuilt `play-local` binary
and its matching `target/debug/deps` artifact to avoid native launch delay. The
native desktop helper has its own build-time signing step.

## Documentation and delivery

```sh
python3 ../docs/tools/check_links.py .
```

Run changed examples against the actual application. For portal delivery, follow
the [shared authoring guide](https://github.com/cubearc/docs/blob/main/content/contributing/documentation.md):
adopt the validated full Play SHA, run docs tool tests/link checks, build clean
pinned sources with `--locked`, review rendered pages and verify the authorized
public deployment's revisions and content.

The current [CI workflow](../../.github/workflows/ci.yml) requires native Rust
Linux/macOS/Windows, Rust quality/coverage, frontend/browser, Linux native workers
and Electron x64/ARM64, macOS Electron ARM64/Intel and Docker integration jobs. Record exact revisions and observed
conclusions; local success does not establish hosted CI success. Replace obsolete
jobs only with reviewed native equivalents. Full local graph/CLI, signed native
packages, updates and per-OS worker/PTY evidence still gate
[final acceptance](https://github.com/cubearc/play/issues/57).
Canonical local package import and private Stage Candidates are tested through
the engine and Electron, including Solution walkthrough revisions. Cloud has an independent
[client/service acceptance gate](https://github.com/cubearc/play-platform/issues/9)
and does not block the local milestone.

Pipeline authoring tests cover explicit creation/revision/fork, stable node IDs,
independent Trail hypotheses, pinned reuse across Explorations, stale edits,
parameter/version/type/ownership rejection and complete Run evidence. A real
schema-8 store and signed Core Run fixture prove schema-9 upgrades preserve
historical evidence. Worker tests exercise saved pixel coordinates and Brainfuck
limits. The real Electron journey edits hypotheses and definitions, reuses a
chosen older revision, checks accessibility and preserves the preceding result.

Typed graph checks retain connections, positions, output selection and exact
revisions; reject cycles, invalid ports, versions and implicit representation
coercions; execute real Core ancestor chains; preserve partial failure evidence;
and enforce per-node retention. A real schema-9 database fixture preserves its
finished Run unchanged during schema-10 migration. The Electron graph journey
adds/connects nodes, moves one by keyboard, executes both under the real worker
policy, inspects their distinct results, promotes the output node and reopens the
same evidence after restart. axe-core runs with the existing rules enabled.

Supervisor lifecycle fixtures use a separate signed, sandboxed test helper that
holds after the canonical node-start event. It makes archive/edit/cancel/Quit
and owner-kill assertions independent of how quickly Core exhausts a finite work
budget. The existing engine wall deadline and worker CPU/VM limits remain in force.
The helper exits when its supervisor dies. The helper fabricates no output and is absent from production
worker selection; actual Core outputs, typed conformance and real Electron Runs
continue to use the normal worker.

## Named Pipeline inputs

The input suite preserves an actual schema-10 store and signed Core Run during
schema-11 migration. It checks two Explorations sharing one graph, distinct pinned
image/caption bindings, explicit Run targets, minimal worker input data, aggregate
text/PNG limits, scoped Pages, archive/restore, stale Trail revisions and exact
copy remapping. The Electron journey authors the graph and bindings through VS
Code Elements, runs both branches, edits/rebinds a caption, reuses the definition
in another Exploration, checks accessibility and reopens the private profile.

## Attached CLI parity

`cube-cli/tests/play_context.rs` exercises the real binary and strict private
frames, including absent context, incompatible versions, duplicate keys, budgets,
retargeting and capture refusal. `desktop/cli.test.ts` runs that bundled CLI
against the real engine and checks Stage/Workspace isolation. Its signed supervision
fixture holds a real node-start boundary to verify CLI start/status/active cancel,
caller conflicts and idempotent retry without fabricating an operation result. The numeric Electron
journey runs the same saved graph through **Run cell** and **Run with Cube CLI**,
compares inputs, definitions and actual outputs, reads/cancels the completed Run
through the CLI, checks reflow and reopens both records after restart.

These checks cover the one-shot attached control client, not external discovery
or Windows worker enforcement. Linux worker evidence comes from its dedicated native job. Unretired Runner/browser
checks and the exact Core operation conformance matrix remain required.

## Contextual native terminal

The native CLI test opens a Rust-owned PTY, reads an exact Asset, changes
its actual terminal dimensions and executes two distinct Runs equal to desktop
results. A held canonical node-start fixture verifies active cancellation. Tests
stop renderer polling to observe lease expiry, kill the actual engine to check
CLI exit, and keep an unrelated process alive. A separate signed probe traverses
the production terminal exec path without reapplying restrictions; private file
reads/writes, networking, children, excess VM and CPU loops remain denied.
Linux runs the same PTY/result/lifecycle tests and applies its actual terminal
policy to adversarial probes, including inherited thread restrictions and
mandatory-policy refusal. Integrity tests reject missing/changed terminal helpers.

The Electron journey uses the terminal input and buttons, verifies explicit Enter
before execution, real Hello evidence, inline Trail refresh, reconnect and close,
800-pixel width and 200% zoom reflow, axe rules and persisted Runs after restart.
It checks terminal console errors separately from axe's blocked measurement styles
and the preexisting VSCode textarea resizer data-URL denial. The actual screen is
included in the terminal guide. These are macOS/Linux development checks, not a claim
that Windows or the broader terminal milestone is complete.

## Parallel branches within a Run

A synchronized Rust test requires two actual execution threads to rendezvous before
invoking Core; it does not infer overlap from elapsed time. Graph/storage tests
cover two simultaneous attempts, the third-node ceiling, reverse independent start
order, duplicate starts, dependency joins, aggregate output limits, successful
sibling evidence after failure, and exact retained results after reopening.

The signed native supervision fixture holds a group after its start events. A
real IPC test observes two persisted running attempts, cancels both without a join
attempt, then uses the production signed worker to compute the joined ROT13 result
and reopen both histories. Existing EOF, deadline, engine-death and unrelated-process
checks remain required. This establishes parallel branches inside one worker,
with both macOS and Linux worker policies. Cross-Run admission is verified below;
Windows native enforcement remains separate.

## Concurrent Run admission

Native IPC fills the two-worker/four-queued capacity across Explorations, refuses
both per-Exploration and aggregate overflow, recovers accepted intents at capacity,
rejects foreign cancellation and cancels queued work without inventing attempts.
It verifies active cancellation, worker failure, owner death and graceful shutdown,
then reopens immutable evidence and computes actual Core results in separate Runs.
Pure queue tests check FIFO within an Exploration, round-robin fairness despite
refills, blocked lanes and cancellation order.

Renderer hook tests hold the IPC boundary to reproduce the former global Run lock
and verify per-Trail duplicate suppression and queue-full recovery. The Electron
queue journey uses a task-owned application copy with the existing signed
supervision helper, original deadlines and unchanged integrity verification. It
observes Running and Queued cells together, cancels the queued cell without
stopping the first, and checks keyboard-accessible controls and axe-core rules.
The normal application and installed worker are not replaced by this fixture.

## Canonical Challenge packages

Engine tests cover grouped image/caption Assets, inert Pages, annotations, binary
files, ordered Stages, source mappings, independent imports, edits and restart.
Malformed JSON/Base64, duplicate IDs/keys, unsupported formats, external paths,
invalid origins, limits and checksum failures reject without partial content.
The real Electron journey selects the documented example through the chooser,
checks cancellation and failure recovery, runs the caption with Core, preserves
reasoning and reopens separate imports. It checks accessibility and zoom/reflow;
a second frame cannot invoke the import bridge.

## Stage Candidates

Engine tests select exact Asset revisions and typed retained outputs, including a
numeric result from the real Core executor. They cover editing/reopening,
Exploration isolation, optimistic conflicts, idempotent receipts, unknown ports,
unretained outputs, revision limits and native schema-13 upgrade preservation.
Database tests reject Candidate/evidence mutation and inject missing output to
check an explicit availability gap with the selected reference intact.

The real Electron journey covers initial empty state through IPC, choosing an
Asset in its canvas, editing its working text without changing the Candidate,
choosing a retained output, stale revision recovery, browsing selection history,
opening the source Run, independent Explorations and application restart. It
checks axe-core rules and shell/panel reflow at the supported small window size
and 200% zoom. None of these checks establishes official answer correctness or
completion of walkthrough export or old runtime retirement.

### Native Solution walkthroughs

`crates/play-engine/tests/solutions.rs` covers immutable explanations and exact
Candidate selections across edits/restart, scoped histories, bounded admission,
idempotency/conflicts, schema 14 upgrade, protected records and unsupported snapshot
format refusal. Real Core graph
execution verifies selected dependency paths excluding alternatives and downstream
nodes. A controlled missing-output fixture preserves the snapshot and reports an
availability gap. Cross-Run cases execute real ROT13 operations, promote and reuse
an output, exclude an unrelated Run, preserve edited-copy ancestry and reopen the
same snapshot. A missing upstream result leaves the selected answer intact,
reports a gap and refuses a partial new revision. No old snapshot format reader is used.

The Electron Candidate/Solution journey saves and revises an explanation, handles
an independently written revision without losing its draft, inspects history,
reopens the app and checks independent Explorations. It also checks accessibility
and notebook reflow at the supported small window and 200% zoom. These checks do
not prove Cloud validation, signed distribution or retirement of the old runtime.
The cross-Run Electron journey reuses a saved result through the notebook,
inspects the contributing Run, reopens the source history and checks axe/reflow
at 200% zoom.

Selected-note tests preserve exact saved revisions and fixed targets across
notebook edits/restart, exclude unselected notes, reject duplicate/missing/foreign
references and retain note text when a linked output is unavailable. The Electron
journey selects Reasoning by keyboard, edits its working text, preserves the earlier
choice through explanation conflicts, reopens it and checks accessibility/reflow.

### Evidence-linked annotations

Engine checks cover exact Asset revisions after edits/copy/archive/restart,
Block identity, finished and failed attempts, unavailable output, invalid ports,
cross-Stage/Exploration isolation, atomic rejected writes, immutable targets and
schema 15 upgrades. The closed IPC parser rejects malformed targets and oversized
UTF-8 notes. The Electron journey creates annotations from the Asset canvas and
retained cell output, preserves an oversized draft for correction, inspects the
original revision after editing, checks source Run navigation, reopens the
profile and verifies independent Explorations. It exercises axe-core and reflow
at the native minimum window size and 200% zoom.
