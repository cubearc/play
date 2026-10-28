# Contributing to Cube Play

Use the [developer quick start](docs/development.md) and keep the trusted Cube checkout beside Play. For the application, follow [source installation](docs/installation.md).

## Quality checks

The repository enforces LF line endings through `.gitattributes`, including on Windows.
Keep that checkout policy so formatters inspect the same bytes on every platform.

Build the launcher assets before Cargo checks. From the root:

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

Browser, Docker, coverage and complete contract-conformance procedures are in [Contributor verification](docs/reference/validation.md). Keep the 80% Rust/frontend coverage gates, denied warnings and exact operation-ID checks intact.

The [CI workflow](.github/workflows/ci.yml) checks out public Cube at a pinned revision over HTTPS; no deploy key or cross-repository secret is required. Report local validation separately from hosted CI; a workflow definition does not establish a passing run.

## Documentation

Follow the [shared authoring guide](https://github.com/cubearc/docs/blob/main/content/contributing/documentation.md). Keep the README focused on first use, task instructions in the user guides and exact API/storage/security contracts in reference pages. Update affected examples and [navigation](docs/SUMMARY.md) with behavior changes; link to shared explanations.

Use `python3 ../docs/tools/check_links.py .` when the docs repository is checked out beside Play. Validate changed commands against the real interface and report the result.

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
