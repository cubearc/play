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

## Private stage candidates

Save a Lab result with a name, then choose **Use result as stage candidate** beside
that result. Select the intended stage and associate it explicitly. Whole text
and byte artifacts must already match the stage's answer type; binary bytes are
never coerced to UTF-8. Image artifacts require an explicit supported result first.

For a structured result choose a JSON pointer to a string field, then either
**String as UTF-8 text** or **Base64 decoded by Core as bytes**. Numeric/boolean
coercions, arbitrary expressions and other conversions are unsupported. For
example `/channels/0` selects a channel name as text; `/preview_png_base64` can
explicitly decode a generated channel PNG to bytes. Core performs the Base64
conversion. The output limit is 64 KiB, with at most 256 retained revisions per
workspace. No expected/private author answer is consulted.

Each association preserves the source artifact, field/conversion choice, Core
version, input/output digests and optimistic revision. Replacing a candidate appends
to history; stale revisions fail instead of overwriting newer choices. The same
SQLite database retains references, while original binaries stay in their existing
bundles. Missing/corrupt bundles retain their candidate history and show recovery
errors. Stage summaries can refresh and restore these associations after restart.

Candidate status is always **private / unverified**. Content digests check storage
integrity, not solving correctness. Selecting/replacing a candidate never submits
anything, marks progress complete, unlocks stages or impersonates official verification.
`GET/PUT /api/challenges/candidates` uses the existing session and workspace boundary.
