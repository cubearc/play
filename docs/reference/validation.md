# Contributor verification

## Quality checks

From the root, build the trusted launcher assets before Cargo checks:

```sh
npm ci --prefix frontend
npm run build --prefix frontend
cargo fmt --all -- --check
rustfmt --check --edition 2024 crates/play-launcher/tests/support/docker_fixture.rs crates/play-launcher/tests/support/folder_picker.rs crates/play-runner/tests/support/resource_probe.rs
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
docker build --target lab-tests --build-context cube_engine=../cube .
```

The suite uses unique Compose projects and available loopback ports, builds the
image, checks HTTP/security/container settings and exercises the real launcher with
empty/private sources, literal paths, concurrent reuse, two workspaces, session
isolation, browser readiness, stop and recreation with persistent test volumes.
Only fixture-owned containers and temporary test volumes are removed. Set
`CUBE_PLAY_CUBE` to a trusted built Cube executable and `CUBE_PLAY_LAUNCHER` to the
Play executable to additionally exercise the real cross-repository CLI handoff.
With frontend dependencies and Chromium installed, the Docker suite runs the full
Playwright suite against the real two-container workspace on desktop/mobile. It
checks known-answer CLI output, errors, PTY resizing/closure, accessibility, previews,
notes/conflicts, exports, RGB/palette counts, transforms/comparison, challenge import,
progress and candidate association. It then recreates the containers and checks
named artifacts, candidate revisions and SQLite recovery. Desktop and mobile use
separate fixture investigations within the production retention limits. Native
browser workers likewise own their temporary server/data directories.
Source hashes must remain
unchanged. The native browser suite intentionally skips its two container-only PTY cases.

Linux Runner integration tests use the real built CLI (or `CUBE_RUNNER_CUBE`) and
exercise PTY sessions, read isolation, output exhaustion and process cleanup.
A test-only executable probes inherited CPU/process/file limits with lower soft
limits after asserting the production values; memory and descriptor exhaustion
are also checked. It runs without a shell and never ships in the runtime image. The Docker `runner-tests`
target reproduces them on macOS/Windows. Native API development does not start a
host terminal; configure a private Linux runner socket explicitly when needed.
The Docker smoke suite also runs a real CLI command while unrelated fixture
processes share its UID, verifying that the container PID budget remains scoped.

Windows background launches use a Windows-only safe process wrapper with an
explicit handle list, since stable `std::process::Command` otherwise inherits
unrelated handles even with standard streams redirected. They do not use a shell
or elevation; this workspace continues to forbid unsafe code. The launcher
integration test requires output EOF within 20 seconds while its chooser remains
alive, then verifies chooser reuse.

Rust integration tests also use a controlled Docker CLI fixture for unavailable
engines, incompatible images, unsafe reuse, ownership failures, bounded output and
process cleanup. These do not substitute for the real Docker suite. The fixture
is compiled by rustc during tests and formatted separately because it is not a
runtime binary target. The configured CI matrix targets native Rust tests on Linux,
macOS and Windows and real Docker integration on Linux. These jobs can also be
reproduced locally; workflow configuration does not establish a passing run.

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

The named-port API and exploration model are documented in
[Typed explorations](../explorations.md). A new or changed operation is complete
only with its canonical Core contract/reference, actual result-conformance tests,
and validated Play adapter/viewer handling. Regenerate contracts in the Core
checkout using `cargo run --locked -p cube-core --example operation_contracts`;
never maintain a second handwritten frontend catalog.
The Lab integration matrix reads Core's packaged synthetic conformance fixtures
and executes every installed ID through the authorized API with real workers.
It rejects missing/duplicate IDs, contract/version drift and incorrect named
outputs, and writes `target/play-operation-conformance.json` for CI. Fixture-owned
investigations are batched within existing retention quotas. File metadata times
are validated but excluded from known-answer equality; floating-point analyses
use a relative tolerance of 1e-12. Integers, decimal strings and bytes stay exact.

On Intel macOS, unsigned rebuilt native binaries can delay worker startup beyond
the integration-test deadline. After building the exact executable under test,
`codesign --force --sign - target/debug/play-local` applies a local ad-hoc signature
without a signing identity or host security-policy change. Rebuilds replace it.
Before rerunning Cargo, also sign its matching executable under `target/debug/deps`;
Cargo can restore the final binary from that artifact.
This is a native development diagnostic; the ordinary launcher uses Linux containers.
The host API and runtime suites also ad-hoc sign their own temporary browser and
folder-picker probes on macOS so host validation does not consume the UI response
deadline.

## Integrated MVP verification

Run the commands above against the final source before release. The runtime suite
uses real Cube CLI delegation when `CUBE_PLAY_CUBE` names the trusted executable;
set it explicitly for full cross-repository verification. Use Rustup's `rustc` on
PATH for the launcher browser-readiness fixture.

| User journey | Reproducible coverage |
| --- | --- |
| Empty/current directory, files, explicit file and browser readiness | Launcher integration and Docker runtime tests |
| Image and MP4 preview or metadata fallback | `workspace.spec.ts`, `lab.spec.ts` |
| Real PTY Cube command and cleanup | `runner.spec.ts`, Linux Runner tests |
| Generate, name, reopen and compare artifacts | `lab.spec.ts`, `notebook.spec.ts`, real Lab API tests |
| Private notes, rapid updates and conflicting tabs | Notebook component/API/browser tests |
| Restart, recreation and crash recovery | Persistence/Lab API tests and Docker runtime tests |
| Non-executable challenge import and local stage progress | `challenges.spec.ts`, real challenge API tests |
| Byte/text/structured-field candidates and retained lineage | `candidates.spec.ts`, real candidate API tests |
| Session, Host/Origin, paths, symlinks, quotas and failed writes | HTTP/workspace/persistence tests and Docker/Runner checks |

The CI matrix includes native Rust tests on Linux, macOS and Windows, plus Linux
quality/coverage, frontend and Docker jobs. For v0.1.0, the maintainer accepts the
recorded local checks and explicitly deferred native Windows/WSL/Docker Desktop
validation on 2026-10-27; retain that work in [the platform follow-up](https://github.com/cubearc/play/issues/24).
Local macOS tests and Linux containers do not establish native Windows validation.
Hosted jobs blocked by account billing/spending limits are accepted as a temporary
integration exception, with no passing hosted result claimed. Keep the workflows,
warnings policy, coverage gates and security checks intact. Record actual commit/run
evidence and scope decisions in the GitHub milestone. The v0.1.0 exception does not
waive functional or conformance requirements for subsequent milestones.
