# Contributing to Cube Play

Use Rust stable through rustup, Node.js 24 and npm. Docker Compose v2 and Python 3.11+
are required for container tests. Start with the [README quick start](README.md#developer-quick-start).

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

## Quality checks

From the root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
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
On Linux, Playwright may require `npx playwright install --with-deps chromium`.

Test the real Compose image and runtime from the root:

```sh
python3 tests/runtime_smoke.py
```

The test uses a unique Compose project and an available loopback port, builds and
starts the image, checks HTTP/security/container settings and always removes its
own containers. It does not stop unrelated containers or remove user data.

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
