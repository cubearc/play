# Local challenge contract

Cube Play v0.1 uses non-executable JSON manifests, identified by
`kind: "cube-play-challenge"` and `format_version: 1`. A `.cube` extension is only
an entry-point hint. It never chooses an executable parser. Core's existing versioned
JSON recipes describe bounded operation pipelines with engine/adapter contracts;
they are not challenge manifests. Proposed functional `.cube` workflows and future
`.cubepack` archives are separate future formats. This MVP neither evaluates them
nor opens archives. Unknown fields (including scripts, commands and private answers)
are rejected. Public packages never need an author's expected answers.

A manifest contains `id`, `title`, optional `description`, `artifacts` and `stages`.
Each artifact has an `id` and relative `path`, resolved beside the manifest in the
same authorized source/import store. Each stage has an `id`, `title`, `instructions`,
optional `steps` (`id`, `instructions`), optional text `hints`, artifact IDs, and an
`answer_type` of `text` or `bytes` (default text). Nothing is fetched remotely.
All workspace roots were already authorized by the launcher; a challenge grants no
new filesystem authority. Import all referenced files first when using managed imports.

Limits: 256 KiB manifest, 16 JSON nesting levels, 4,096 nodes, 32 stages, 16 steps and
16 hints per stage, 32 artifact references, 8 KiB per instruction/hint, 32 MiB per
referenced artifact and 64 MiB aggregate referenced size. IDs are unique ASCII
letters/digits/underscore/hyphen, 1–64 bytes. Titles are 1–512 UTF-8 bytes.
Duplicate JSON keys, IDs/references, unknown versions, missing files, traversal,
symlinks and special files fail before any challenge/progress transaction commits.

Import explicitly from a selected manifest, or launch `cube play rabbit.cube`.
Instructions and hints are inert text. Artifact buttons use the existing explorer,
Lab and terminal mounts. Stages have no hidden unlock or verification rule. Navigation
and locally marked progress persist in the same SQLite investigation; they never
represent official verification. Challenges remain optional for all Free Lab flows.
Core hashes of manifest content and its relative local store/path identify revisions, so editing a public definition creates
a separate local challenge/progress namespace and preserves the previous one.
At most 16 definitions are retained per workspace. Reopening the same content reuses
its existing selected stage and progress. Missing artifacts after import retain their
references and fail clearly in the file inspector.

The original [rabbit.cube](../tests/fixtures/rabbit.cube) fixture is a complete
working example beside [sample.png](../tests/fixtures/sample.png). It contains no
answer. Copy both files to an empty directory and run `cube play rabbit.cube`.
