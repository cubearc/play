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

CI runs the Electron proof on macOS ARM64 and Intel. The macOS worker is locally
ad-hoc signed and sandboxed. Windows/Linux Rust tests and protocol checks do not
prove worker/PTY enforcement; unsupported native execution must refuse. None of
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
1e-12. File metadata times are validated separately from known-answer equality.
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
Linux/macOS/Windows, Rust quality/coverage, frontend/browser, macOS Electron
ARM64/Intel and Docker integration jobs. Record exact revisions and observed
conclusions; local success does not establish hosted CI success. Replace obsolete
jobs only with reviewed native equivalents. Full graph/CLI/Cloud, signed native
packages, updates and per-OS worker/PTY evidence still gate
[final acceptance](https://github.com/cubearc/play/issues/57).
