# Contributing to Cube Play

<a id="changes-and-review"></a>

Start with [Development](docs/development.md) to build the Electron workbench and
[installation](docs/installation.md) to try the source preview. Keep the trusted
Cube checkout beside Play. The [architecture](docs/architecture.md) describes
implemented native behavior and the explicit remaining repository transition.

## Quality checks

Run [Contributor verification](docs/reference/validation.md), including native
engine/Electron checks, strict Rust/TypeScript/Biome, real operation conformance
and unchanged 80% coverage gates. Keep LF line endings on every OS. Existing
browser/Runner/Docker checks remain required while their production paths still
exist; they are not prerequisites for opening Electron. Replace each check only
with validated native coverage when its implementation is retired.

[CI](.github/workflows/ci.yml) uses an explicit public Cube revision and records
native Rust, quality/coverage, frontend, macOS Electron and remaining Docker jobs.
Report exact revisions and observed local/hosted results separately. A workflow
definition or local helper signature does not establish signed release support.
Do not suppress warnings, skip supported operation IDs or lower coverage gates.

## Documentation

Follow the [shared authoring guide](https://github.com/cubearc/docs/blob/main/content/contributing/documentation.md).
Keep README and task guides aligned with actual native behavior. Put exact
contracts in reference pages and update [navigation](docs/SUMMARY.md). Preserve
published URLs with concise pointers when consolidating explanations.

Run `python3 ../docs/tools/check_links.py .` with the sibling docs checkout.
Validate changed commands through the real application. After a product commit
passes its required checks, adopt that full SHA in `cubearc/docs/sources.lock.json`,
build clean pinned sources and verify the authorized public portal deployment.

## Changes and delivery

Write documentation, comments, errors and issue descriptions in English. Use
Conventional Commits and focused changes. Preserve shared history; do not rewrite
it without authorization. Follow the user's authorized branch/commit/publication
scope. No PR is needed when the user explicitly requests direct main commits.

Algorithms and operation metadata belong to Core. Play owns presentation, scoped
local state and execution orchestration. Follow the
[desktop domain contract](docs/reference/desktop-domain.md) for immutable evidence,
Workspace/Exploration ownership and mandatory retirement. Each replacement must
remove its superseded interface and validate the observable native behavior.

GitHub Issues and CubeArc Roadmap own delivery status. Do not add a repository
backlog. Close issues only after their actual acceptance criteria, required CI and
authorized integration/publication pass; keep broader incomplete issues open.
