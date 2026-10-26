# Contributing to Cube Play

Use Rust stable through rustup, Node.js 24 and npm. Docker Compose v2 and Python 3.11+
are required for container tests. Start with the [README quick start](README.md#developer-quick-start).
The engine must support Landlock ABI 3 with its LSM enabled; use Docker Desktop
4.94.0 or newer on macOS/Windows. Unsupported isolation fails before starting a
terminal. See [terminal boundaries](docs/terminal.md).
Keep a trusted Cube 0.18.0 checkout at `../cube`; Cargo consumes its exact-version
library directly. CI checks out a pinned Cube revision beside Play. While Core is
private, hosted CI requires a read-only Cube deploy key in `CUBE_CORE_READ_KEY`,
configured by a maintainer. No key enters application containers or build contexts.
Source access and GitHub Actions billing are prerequisites for hosted checks;
passing local tests do not imply CI passed.

## Development

For frontend hot reload, run these commands in two terminals from the root:

```sh
CUBE_PLAY_ORIGIN=http://127.0.0.1:5173 cargo run --locked -p play-local
npm run dev --prefix frontend
```

In PowerShell, set `$env:CUBE_PLAY_ORIGIN = 'http://127.0.0.1:5173'` before the Cargo
command. Open the printed session URL after Vite is ready. Vite proxies local API
requests to port 7331 and preserves the browser host. Both listeners use loopback.
To test production serving, build the frontend and start the backend without the
development origin override.
To exercise files, use the launcher or configure both `CUBE_PLAY_SOURCE` and
`CUBE_PLAY_WORKSPACE` (a regular JSON file with a v4 UUID `id` and display `name`).
Set `CUBE_PLAY_DATA` to enable the persisted file workspace and imports; SQLite uses its sibling `state/`
directory, or an explicit `CUBE_PLAY_STATE` override. See [local persistence](docs/persistence.md)
for private directory permissions, migration rules and recovery. The browser harness manages its own
temporary fixtures automatically.

## Quality checks

From the root:

```sh
cargo fmt --all -- --check
rustfmt --check --edition 2024 crates/play-launcher/tests/support/docker_fixture.rs crates/play-runner/tests/support/resource_probe.rs
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --locked --manifest-path ../cube/Cargo.toml -p cube-cli
cargo test --workspace --all-features --locked
npm ci --prefix frontend
npm run check --prefix frontend
npm run build --prefix frontend
npm run test:coverage --prefix frontend
cargo build --workspace --locked
```

For real desktop/mobile browser smoke tests, run from `frontend/`:

```sh
npx playwright install chromium
npm run test:e2e
```

The harness starts the compiled Rust server on loopback port 17331 and serves the
production frontend. It checks the real session/API flow, refresh, browser errors,
responsive overflow and external requests. No application API is mocked in these
tests. Browser traces are disabled to avoid capturing session capabilities.
It also tests Core-backed previews, inert hostile text, unsupported media, single-file
selection, navigation, byte ranges, imports, saved-state restoration and conflicting
tabs. Browser cases run serially because the real server owns one investigation.
On Linux, Playwright may require `npx playwright install --with-deps chromium`.

Test the real Compose image and runtime from the root:

```sh
python3 tests/runtime_smoke.py
docker build --target runner-tests --build-context cube_engine=../cube .
```

The suite uses unique Compose projects and available loopback ports, builds the
image, checks HTTP/security/container settings and exercises the real launcher with
empty/private sources, literal paths, concurrent reuse, two workspaces, session
isolation, browser readiness, stop and recreation with persistent test volumes.
Only fixture-owned containers and temporary test volumes are removed. Set
`CUBE_PLAY_CUBE` to a trusted built Cube executable and `CUBE_PLAY_LAUNCHER` to the
Play executable to additionally exercise the real cross-repository CLI handoff.
With frontend dependencies and Chromium installed, the Docker suite also runs
`npm run test:runner` against its real two-container workspace on desktop/mobile.
It checks known-answer CLI output, errors, PTY resizing, accessibility and closure.
The ordinary browser suite reports these container-only cases as skipped.

Linux Runner integration tests use the real built CLI (or `CUBE_RUNNER_CUBE`) and
exercise PTY sessions, read isolation, output exhaustion and process cleanup.
A test-only executable probes inherited CPU/process/file limits with lower soft
limits after asserting the production values; memory and descriptor exhaustion
are also checked. It runs without a shell and never ships in the runtime image. The Docker `runner-tests`
target reproduces them on macOS/Windows. Native API development does not start a
host terminal; configure a private Linux runner socket explicitly when needed.

Rust integration tests also use a controlled Docker CLI fixture for unavailable
engines, incompatible images, unsafe reuse, ownership failures, bounded output and
process cleanup. These do not substitute for the real Docker suite. The fixture
is compiled by rustc during tests and formatted separately because it is not a
runtime binary target. Native Rust tests run on Linux, macOS and Windows; real
Docker integration runs on Linux CI and can be reproduced on Docker Desktop.

## Coverage

```sh
cargo install cargo-llvm-cov --version 0.9.1 --locked
cargo llvm-cov --workspace --all-features --locked --fail-under-lines 80 --ignore-filename-regex '/tests/'
cargo llvm-cov report --lcov --output-path lcov.info --ignore-filename-regex '/tests/'
```

The Rust line gate is 80% of application source, including process startup; test
sources are excluded. This keeps request/security/configuration and runtime paths
covered without counting test assertions as product coverage. Frontend gates are
80% of statements, branches, functions and lines; the root mount is exercised by
real browser tests. CI prints actual coverage and uploads LCOV artifacts without
requiring an external reporting account. Improve tests rather than lowering gates
or suppressing warnings. Do not commit generated coverage/build output.

## Changes and review

Use concise English for code comments, errors, docs, issue descriptions and templates.
Follow Conventional Commits: `type(scope): imperative description` with optional
scope. Keep commits atomic and branches/PRs focused on one capability, including
meaningful tests and documentation for the changed behavior. Reference the GitHub
issue. Never rewrite shared history without authorization.

Algorithms belong in `cube-core`; Play owns presentation, local state, filesystem
authorization and orchestration. Read [Architecture](docs/architecture.md) before
changing these boundaries. Avoid unused abstractions, speculative services and
duplicate metadata catalogs. Use existing tools instead of custom quality wrappers.

Keep planning in GitHub Issues and CubeArc Roadmap. After authorized delivery,
verify remote commits and CI. Close an issue only after its acceptance criteria pass
and the work is integrated into the target branch; keep incomplete work open.
