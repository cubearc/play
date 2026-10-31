# Cube Play repository instructions

- Use stable Rust edition 2024 and strict TypeScript/React. Keep modules small,
  dependencies minimal and errors explicit. Avoid unused scaffolding.
- Follow the [native architecture](docs/architecture.md) and
  [desktop domain contract](docs/reference/desktop-domain.md). The Electron
  renderer is a presentation adapter; the Rust engine owns scoped domain,
  private storage and Run supervision. Algorithms and operation metadata belong
  to versioned `cube-core`; native CLI/PTY integration runs the real Cube CLI.
- For changes to user-facing UI, interactions, navigation, copy or visual
  presentation, read and apply the
  [Cube/Play UX/UI skill](.agents/skills/cube-play-ux-ui/SKILL.md)
  before implementation and during review. Apply it to feature work and fixes,
  even when the request does not explicitly mention design. Respect review-only
  and planning-only scope.
- Every changed operation needs its canonical Core descriptor, named schemas,
  generated reference, real conformance fixture and tested Play adapter/viewer.
  Catalog presence does not grant native execution. Preserve direct-only
  capabilities and exact installed operation-ID/version/schema conformance.
  Never duplicate algorithms or metadata in the frontend.
- Preserve renderer sandbox/context isolation, exact trusted-frame IPC authority,
  private pipes, OS-held storage ownership, read-only sources and bounded native
  processing. Workers receive minimum inputs, no database/credential access.
  Refuse unavailable native enforcement; no unrestricted/container fallback,
  implicit network reporting or automatic Challenge execution.
- Ship only the native Electron/engine runtime. Keep the reviewed retirement
  boundary enforced by tests/architecture_test.py. Unsupported stores/formats are
  refused without writes; never add converters, browser servers or container
  fallbacks. Keep native worker/PTY, exact Core conformance and coverage checks.
- Write docs, comments, errors, templates and issue descriptions in English.
  Update task/reference docs and [navigation](docs/SUMMARY.md) with functionality.
  Follow the [shared authoring guide](https://github.com/cubearc/docs/blob/main/content/contributing/documentation.md),
  adopt validated product revisions in the portal and verify authorized publication.
- Follow [CONTRIBUTING](CONTRIBUTING.md) and
  [verification](docs/reference/validation.md). Keep strict warnings, exact real
  conformance and unchanged Rust/frontend 80% coverage gates. Native checks
  do not establish signed releases or unsupported operating-system enforcement.
- Use focused Conventional Commits and preserve history. Follow the user's
  authorized branch/commit/push scope; direct main delivery needs no PR when
  explicitly authorized. Verify remote commits and required CI before closing
  issues. Keep planning in Issues and Roadmap, not a repository backlog.
