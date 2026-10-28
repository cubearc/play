# Cube Play repository instructions

- Use stable Rust edition 2024 and strict TypeScript/React. Keep modules small,
  dependencies minimal and errors explicit. Avoid unused scaffolding.
- All algorithms and operation metadata belong to versioned `cube-core`. Play owns
  UI, filesystem authorization, local persistence and execution orchestration.
  The terminal runs the real Cube CLI; the Lab uses typed Core APIs.
- Every new or changed operation needs its canonical versioned Core descriptor,
  named input/output schemas, generated reference, real conformance fixture and
  tested Play adapter/viewer handling. Catalog presence does not grant execution.
  Preserve direct-only operations and never duplicate metadata in the frontend.
  The real API conformance matrix must execute every installed catalog ID and
  match the reviewed Core fixture/version/schema identity; no count threshold.
- Preserve loopback exposure, Host/Origin checks, local session authorization,
  read-only sources, bounded processing and no implicit network reporting. Never
  add a Docker socket, privileged container or automatic challenge execution.
- Write documentation, comments, errors, templates and issue descriptions in English.
  Update concise user/developer docs alongside functionality.
- Follow the [shared documentation guide](https://github.com/cubearc/docs/blob/main/content/contributing/documentation.md).
  Keep user tasks separate from API/storage reference and retain one canonical
  explanation per subject. Update `docs/SUMMARY.md` and affected examples.
- Follow CONTRIBUTING.md for Rustfmt, Clippy with denied warnings, locked Cargo
  tests/coverage, Biome, strict TypeScript, Vitest, Playwright and Docker smoke tests.
  Do not suppress warnings or lower coverage gates to bypass checks.
- Use Conventional Commits, small atomic changes and focused branches/PRs. Preserve
  history. Commit completed logical units when commits are authorized; verify remote
  commits/CI after authorized pushes. Do not close issues before validated integration.
- GitHub Issues and CubeArc Roadmap own planning. Do not add a repository backlog.
