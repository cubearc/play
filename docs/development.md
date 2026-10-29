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

macOS executes the installed native text adapters. Windows/Linux refuse worker
execution until their policies are validated; a shell preview does not establish
release support. See [native execution](reference/desktop-runtime.md).

## Explore the notebook preview

Follow [your first notebook](getting-started.md), then [add a PNG with its
caption](workspace.md). Use a second Exploration to check independent content,
reasoning and selected Stages. Quit/reopen and inspect retained output and earlier
Runs in both the notebook and caption canvas. Choose not to retain an output to
check the explicit history gap. The image canvas supports preview and zoom.

The real Electron smoke test covers image/caption grouping, inline results,
Explorer resizing, keyboard navigation, two Explorations, native input ownership,
reasoning and reopening. Its chooser fixture selects a real PNG through the
trusted import handler. It does not establish arbitrary graph execution or Cloud
participation. Screenshots are written under `frontend/playwright-report/`.

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
