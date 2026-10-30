# Develop Cube Play

<a id="containers"></a>
<a id="developer-quick-start"></a>
<a id="development"></a>
<a id="electron-boundary-proof"></a>

Use the exact Rust toolchain in `rust-toolchain.toml`, Node.js from
`frontend/.nvmrc` and npm. Keep trusted Cube 0.18.0 sources at `../cube`; CI records
the reviewed Core revision. On macOS, install Xcode Command Line Tools for the
native helper build and local ad-hoc signing.

## Build and validate Electron

From the Play repository root:

```sh
npm ci --prefix frontend
npm run desktop:build --prefix frontend
npm run desktop:test --prefix frontend
npm run desktop:smoke --prefix frontend
npm run desktop:proof --prefix frontend
```

The build compiles strict desktop TypeScript, bundles the React renderer/preload
and builds the Rust engine/native helper. `desktop:test` exercises real engine
protocol, storage/Run behavior and native capability boundaries. `desktop:smoke`
launches the actual Electron app with temporary private data; `desktop:proof`
opens it interactively. The macOS helpers use local ad-hoc signatures, without a
publisher certificate or host policy changes.

The Electron preview enables native execution on macOS. Linux worker validation
runs separately below; Windows workers remain unavailable. A shell preview does
not establish release support. See [native execution](reference/desktop-runtime.md).

## Validate Linux workers

On a native Linux x64 or ARM64 host with Landlock ABI 3 and seccomp enabled, run
from the Play root after `npm ci --prefix frontend`:

```sh
cargo build -p play-engine --bin play-engine --examples --locked
mkdir -p frontend/desktop-native
cp target/debug/play-engine target/debug/examples/native-boundary-probe target/debug/examples/linux-boundary-probe target/debug/examples/native-supervision-probe frontend/desktop-native/
(cd frontend && npx tsc -p tsconfig.desktop.json)
(cd frontend && node --test --test-concurrency=1 desktop-dist/linux.test.js desktop-dist/registry.test.js desktop-dist/workspace.test.js)
```

These checks execute actual Core operations and retained Runs through native
processes, then verify policy denial, resource limits and supervisor cleanup.
They require no Electron launch, container, root privilege or host policy change.
The test binaries are development fixtures; they are not application IPC actions.
This worker validation does not enable the Linux Electron preview or terminal.
The [runtime reference](reference/desktop-runtime.md#linux-worker-boundary) owns
kernel requirements and exact limits.

## Explore the notebook preview

Follow [your first notebook](getting-started.md), then [add a PNG with its
caption](workspace.md). Use a second Exploration to check independent content,
reasoning and selected Stages. Quit/reopen and inspect retained output and earlier
Runs in both the notebook and caption canvas. Choose not to retain an output to
check the explicit history gap. The image canvas supports preview and zoom.

The real Electron smoke test covers image/caption grouping, inline results,
Explorer resizing, keyboard navigation, two Explorations, native input ownership,
reasoning and reopening. It also reorders every Block kind with pointer/keyboard,
checks stale-order recovery with a preserved draft, and verifies focused-cell
execution and immutable earlier Run evidence. Its chooser fixture selects a real PNG through the
trusted import handler. It does not establish arbitrary graph execution or Cloud
participation. Screenshots are written under `frontend/playwright-report/`.
The duplication journey checks independent text/caption edits and Runs, source
revision conflicts, keyboard copy, grouped images, quota state and restart.
Native tests also preserve copy provenance across schema 2 upgrades and refuse
lineage changes; engine tests cover atomic rollback after a caption quota failure.

Use **Add file** to import a small text or binary fixture. Its Block opens an
inert inspection canvas with a byte count, bounded preview and digest. The file
journey exercises HTML as text, binary preview, chooser cancellation, independent
copies, keyboard order, archive/restore, Exploration isolation and restart.
Native checks also reject oversized/special files and renderer path authority;
source file bytes remain unchanged.
Save a retained output with **Save as block**, then run its text independently
and follow **Open source Run** after editing/re-running the original clue. The
result journey verifies exact historical selection, keyboard focus, preserved
reasoning drafts, read-only roots, archive/restore, isolation and restart. Native
IPC also promotes actual Brainfuck bytes without text coercion; engine tests
cover unavailable outputs, conflicts, quota rollback, unique origins and schema-5
upgrade. Core determines the operation version and output contract in these checks.
Archive journeys cover preserved history and drafts, keyboard restoration,
image/caption grouping, active-slot reuse, retained limits and independent
Explorations. Native tests archive an admitted worker without stopping its
supervision, then cancel it and reject new work until restoration. Schema fixtures
reconstruct exact earlier native stores with their real domain records and verify
protected evidence after upgrade.

Use **Add annotation** for narrative text independent of Asset operations. The
annotation Electron journey checks per-Note keyboard saves, separate reasoning
drafts, inert markup, revision-conflict recovery and discard, keyboard copy/order,
archive/restore, Exploration isolation and restart. Engine checks cover the same
Note revision storage, stale snapshots, byte/capacity admission, immutable source
lineage and protected reasoning, including schema-6 upgrades with saved evidence.

The complete notebook journey combines inert Page source, grouped image/caption,
File, annotation and Result Blocks. It uses keyboard creation/edit/copy/order/
archive/restore, selects compatible image and caption operations, checks exact
image Asset binding and RGB bytes, switches Explorations and reopens all saved
identities. Hostile page scripts/forms/embeds stay text. axe-core runs in the real
Electron document with WCAG 2 A/AA and 2.1 AA rules for notebook and image canvas;
no rules are disabled. Tests cover tab arrow/Home/End navigation, readable
contrast and keyboard access to the image viewport. Native IPC also saves actual
pixel bytes as a File result and rejects archived-image dispatch; schema-7
fixtures preserve Notes and bind prior image Trails without losing evidence.

## Source boundaries

- `frontend/desktop-renderer-src`: VSCode Elements workbench and presentation.
- `frontend/desktop`: main/preload, typed bridge, engine process and Electron tests.
- `desktop/protocol.json`: shared wire and resource limits.
- `crates/play-engine`: domain commands, SQLite, supervisor and native worker.
- `../cube/crates/cube-core`: operation algorithms, metadata and contracts.

Keep domain decisions in Rust, algorithms in Core and interface code as adapters.
The renderer may request a scoped Asset revision, not a path or SQL query.
[Architecture](architecture.md) and the [domain contract](reference/desktop-domain.md)
define those boundaries. Do not add another operation catalog to the frontend.

## Required verification

[Contributor verification](reference/validation.md) is the canonical command list
for native checks, coverage, exact operation conformance and remaining repository
checks. The old web/container implementation still has required tests pending
its replacement; running it is not part of trying the Electron app.

## Maintain guide screenshots

Use real macOS Electron captures with disposable example data. The native smoke
writes PNGs to `frontend/playwright-report/`; capture additional guide states
through the actual app. Keep sources and private profiles outside published
images. Check that the full figure, operation and result are visible at a readable
window size or zoom.

Store reviewed PNGs in `docs/assets/desktop/`. Add descriptive Markdown alt text, a
short caption explaining the visible state and a link to the full-size PNG. Update
affected captures with visible UI changes. The portal copies these assets from
the pinned product revision; review their rendered desktop/mobile layout and
verify the published image bytes alongside the page.

Pipeline authoring tests cover explicit creation/revision/fork, stable node IDs,
independent Trail hypotheses, pinned reuse across Explorations, stale edits,
parameter/version/type/ownership rejection and complete Run evidence. A real
schema-8 store and signed Core Run fixture prove schema-9 upgrades preserve
historical evidence. Worker tests exercise saved pixel coordinates and Brainfuck
limits. The real Electron journey edits hypotheses and definitions, reuses a
chosen older revision, checks accessibility and preserves the preceding result.

Named-input tests retain an actual schema-10 signed Run while upgrading to schema
11. The complete Electron journey declares and binds image/caption inputs, runs
different targets from one graph revision, preserves a pinned caption during
editing, explicitly rebinds it and reuses the graph in another Exploration. It
captures `native-pipeline-inputs-editor`, `native-pipeline-inputs-canvas` and
`native-pipeline-inputs-binding` for the guide.

The registry journey captures `native-spiral-editor` and `native-spiral-canvas`
for `spiral-editor.png` and `spiral-canvas.png`. It also checks the long contract
form at the minimum 800×540 window and 200% zoom, keeping the save action visible
and shell status within the viewport. These are real Core/worker Runs and exact
restart assertions, not fabricated operation or Cloud results.
