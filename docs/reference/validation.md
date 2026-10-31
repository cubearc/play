# Contributor verification

Validate the native application, its current storage baseline and the real Core
contracts. Run commands from the Play checkout with the trusted Cube checkout
beside it. Use [development](../development.md) for setup and
[standalone packaging](../installation.md#build-a-standalone-preview-bundle)
for an actual bundle. Keep production worker limits and coverage gates unchanged.

## Native engine and Electron

```sh
python3 tests/architecture_test.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
npm ci --prefix frontend
npm run check --prefix frontend
npm run typecheck --prefix frontend
npm run build --prefix frontend
npm run desktop:test --prefix frontend
npm run desktop:smoke --prefix frontend
```

The Rust suite checks explicit Workspace/Exploration ownership, independent
notebooks, immutable Assets/Notes/Pipelines/Runs, ordered and grouped Blocks,
archive/restore, exact Candidate/Solution evidence, selected Notes, cross-Run
lineage, quotas, stale writes, retry receipts and restart/recovery. Cutover tests
reject prior schema versions and conversion markers without changing archived
bytes or permissions. Fresh stores initialize the one current schema directly.

Native host tests exercise trusted-frame IPC, inert resources, exact binary
integrity, bounded worker/PTY policies, source/file/network denial, cancellation,
queue admission and owned process-tree cleanup. Platform-specific tests run on
their native hosts; skipped or unavailable Windows enforcement remains a gap.
No container or unrestricted worker substitutes for a refused native policy.

The real Electron journeys cover two independent Explorations, image and caption
in one Block, inert text/file/Page/Note content, revision and draft conflicts,
keyboard focus/reflow, graph and declared-input editing, multiple targets,
UI/real Cube CLI equivalence, bounded branches/queued Runs, contextual terminal,
Challenge import and selected Candidate/Solution/annotation lineage after restart.
Screenshots and native conformance evidence go to `frontend/playwright-report/`.

## Coverage and Core conformance

```sh
cargo llvm-cov --workspace --all-features --locked --fail-under-lines 80 --ignore-filename-regex '/tests/'
cargo test --locked --manifest-path ../cube/Cargo.toml -p cube-core --test conformance_matrix --test operation_contracts --test execution_registry
npm run test:coverage --prefix frontend
```

The frontend command combines current workbench unit tests with V8 coverage from
real Electron journeys. Both producers share one source mapping and V8
conversion. The command verifies a matching bundle digest, source maps and the
presence of every production renderer source before enforcing **80% lines,
functions, statements and branches**. Collection happens in the test driver;
no coverage authority or counters are exposed by production IPC. Packaged smoke
runs use the ordinary bundle with no coverage collection or shipped source maps.

Core's complete fixture matrix executes every installed operation ID and verifies
its actual contract/result. The native registry independently discovers eligible
executors and checks each through owned IPC and the enforced real worker against
Core known answers. Catalog presence alone never implies native execution.
Keep the full catalog/fixture matrix and native capability checks; do not hardcode
an operation count or replace actual results with shape-only mocks.

## Qualify an actual bundle

After the native build, create a preview in an empty destination:

```sh
npm run desktop:package --prefix frontend
PLAY_PACKAGED_EXECUTABLE="/absolute/path/to/the/bundled/executable" npm run desktop:smoke --prefix frontend
```

Use the executable declared by the bundle. On Linux, configure the Chromium
sandbox helper in that copied bundle as described in installation, and use a
space-free path. The same journeys verify packaged mode with no development
runtime PATH, source checkout or Docker/WSL requirement. Packaging rejects missing
policies, mismatched native digests and occupied destinations. Check that sources,
source maps, test probes, dependencies and private stores are absent from the app.
The controlled queue fixture is explicitly test-only and cannot ship in a preview.

A local receipt or helper signature does not prove publisher signing, notarization,
authenticated updates or installed lifecycle support. Record the exact source,
OS/architecture, bundle receipt, executed checks and unavailable tools separately.

## Documentation and delivery

```sh
python3 ../docs/tools/check_links.py .
```

Read changed guides against the actual native commands and UI. After product CI
passes, adopt its full clean SHA in the portal's `sources.lock.json`, run portal
tool/link checks and a locked build, then verify authorized deployment and public
source identities/assets. Preserve existing worktrees and published documentation
URLs. Keep issue/milestone acceptance open for actual unsupported platforms,
remaining product capabilities and signing/update/lifecycle gaps.
