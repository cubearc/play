# Native workspace storage

The Rust engine owns each managed Workspace, its independent Explorations, local
Stages, ordered content Blocks, Assets, Trails and reusable Pipeline definitions.
The [first notebook](../getting-started.md) shows creation, reasoning and
reopening; [Asset imports](../workspace.md) covers PNG selection. The
[domain contract](desktop-domain.md) defines the complete product model; this
reference describes the implemented storage slice.

## Identity and ownership

The app uses `CubeArc/Play` inside Electron's per-user application-data directory.
A trusted `--user-data-dir` development launch may choose a separate absolute
profile. The renderer cannot choose a storage path. Stores live under its
`workspaces` directory, outside source files:

```text
workspaces/
  creation.lock
  recent.json
  <engine-generated-workspace-uuid>/
    workspace.json
    workspace.sqlite3
    owner.lock
```

`workspace.json` binds product identity, generation, Workspace UUID, title and
creation intent. A filesystem move never changes the UUID. `recent.json` records
the last successfully opened Workspace. An OS-held exclusive lock on `owner.lock`
protects the writable connection. Another engine receives `workspace_busy` while
that owner lives; killing it releases the OS lock. The lock file remains in place.
Connection shutdown precedes lock release. The development app has one active
Workspace and one trusted window; multiwindow and contextual CLI attachment are
separate deliveries.

On Unix, private directories require the current user and mode 0700; files require
that user, mode 0600 and a single hard link. Symlinks and nonregular files are
rejected. These checks and file locking are storage controls, not a worker sandbox
or proof of the Windows/Linux release security boundary.

## Single baseline and refusal

The native store uses generation **2**, schema version **2**, SQLite application ID
`0x4355504e` and 4096-byte pages. The native baseline is in `crates/play-engine/src/workspace/schema.sql` and its
accepted generation evolves through `runs.sql`. Recognized native schema 1
upgrades to schema 2 under its owner lock, preserving content and IDs. The engine
recovers the recognized source, closes its connection, upgrades a private copy,
verifies it and publishes it by atomic rename. A validated Workspace-bound
`upgrade.json` marker identifies that unpublished copy; a new owner discards
only its marked copy/journal before retrying. Unknown markers refuse opening
without database writes or cleanup.
Fresh stores initialize that same baseline/evolution. No previous product-store
reader, converter or automatic reset is installed. The engine checks the immutable marker, raw SQLite
header, complete schema signature and stored identity through a read-only immutable
connection before writable recovery or migration under the owner lock. Foreign-key integrity
is then checked. WAL/SHM files are unsupported; this baseline uses rollback
journaling.

An unknown generation, altered schema, damaged header or unsafe store is refused.
The app preserves its files and reports the failure. It never silently creates a
replacement database at the old identity. Create a separate Workspace or use the
version that supports the existing store.

Creation initializes an unpublished `.creating-<uuid>` directory, syncs it and
publishes it by rename. Retrying the same creation key/content returns the same
published Workspace; different content with that key is rejected. Interrupted
initializations are never adopted or reset automatically and consume the managed
Workspace quota. Only an initialization created by the current failed command
may be discarded before publication.

## Scoped records and immutable revisions

Composite foreign keys bind Stage, Note, Block, Asset reference and Trail to their
Workspace and Exploration. Two Explorations may use the same Workspace Pipeline
without sharing notes, content or navigation. Each remembers its selected Stage.
Images and their captions occupy one Block while retaining distinct Asset UUIDs
and exact revisions. PNG bytes are copied into SQLite; imports never write their
source file. Main obtains the file through a native chooser; the engine validates
it with Core's bounded image decoder.

Asset, Note and Pipeline revision rows are immutable. Saving reasoning supplies
the Note's expected revision and appends a revision atomically. Stale edits receive
`state_conflict`; the renderer keeps the draft. Per-command receipts bind an intent
UUID to its payload digest and committed response. Matching retries do not create
duplicate records; changed payloads receive `idempotency_conflict`.

Text Assets create a reusable, single-node ROT13 Pipeline definition using the
actual Core descriptor/version. Trails bind their own source Asset revision.
Changing the operation selector is a draft for the next Run. Accepted executions
freeze their chosen definition, resolved inputs and Core contract; see
[native PipelineRuns](desktop-runs.md) for state, named outputs, private retention
and interruption recovery. Candidates, Solutions, submissions and official
evaluation are separate deliveries.

## Block ordering

Stage views return one ordered `blocks` sequence tagged with `kind: asset` or
`kind: note`. The reasoning Note is a Block with its own ID and revisioned Note
content; there is no separate `Stage.note` field. Image and caption belong to one
Asset Block. The current baseline has exactly one reasoning Block per Stage.

The closed `reorder_blocks` intent carries `context`, `key`, `expected_revision`
and `block_ids`. Supply every Block UUID in that Stage exactly once, including
reasoning. The engine rejects omitted, duplicate, foreign or caption Asset IDs
with `invalid_order`, and stale Stage revisions with `state_conflict`. Preload
also checks UUIDs, uniqueness and the eight-Block ceiling before forwarding.

One transaction changes positions and advances the Stage revision. An identical
order is a no-op; its revision stays the same. Mutation receipts preserve accepted
responses for matching retries. The command never revises Assets, Notes, Trails,
Pipelines or Runs. A failed move changes nothing. Reopening returns the saved
sequence, independently for every Stage and Exploration.

Stage revisions track structural membership and order. Adding text or an image
also advances this revision; editing content or saving reasoning uses its own
revision. Content is inserted before a trailing reasoning Block, otherwise at the
end. No new schema is required: all Block positions live in native schema 2.
The coordinated app/engine wire protocol is **5**, with no older DTO fallback.

## Text and caption revisions

The closed `revise_text` intent carries `context`, `key`, `block_id`, `asset_id`,
`expected_revision` and `text`. The engine resolves a text Asset actually bound
to that Stage Block, checks its current and latest revision, validates UTF-8 byte
limits and charges retained bytes before appending. Image Assets are not editable
through this intent. Another owner's IDs or an unrelated Block/Asset pair fail.

One transaction appends immutable content/digest, advances the Block's matching
Asset or caption reference and advances matching working Trail bindings in that
Stage. It preserves IDs, the image/figure and every accepted Run/attempt/input
reference. Equal text at the expected revision is a no-op; a stale revision is
still a conflict. Expected revisions must be positive and cannot overflow.

Mutation receipts make identical retries return the original accepted response;
a changed payload with that intent key fails. A conflict or quota failure leaves
prior content and evidence intact. A running worker keeps its already frozen
input. New Run admission separately checks its explicit expected Asset identity
and revision; reading a newer notebook never rewrites a completed Run.

## Development ceilings

[`desktop/protocol.json`](https://github.com/cubearc/play/blob/main/desktop/protocol.json)
is the shared source for these limits. Counts reject the entire mutation; text
lengths measure UTF-8 bytes, and historical revisions consume storage.

| Resource | Maximum |
| --- | --- |
| Managed Workspaces, including interrupted initializations | 16 |
| Explorations per Workspace | 32 |
| Stages per Exploration | 16 |
| Blocks per Stage, including the reasoning block | 8 |
| Text Asset or image caption | 1024 bytes |
| One reasoning revision | 2048 bytes |
| One imported PNG | 131072 bytes (128 KiB) |
| Asset, reasoning and Run payloads/reservations per Exploration | 4194304 bytes (4 MiB) |
| Runs per Exploration / Workspace | 64 / 128 |
| Pending Run output reservation | 131072 bytes (128 KiB) |
| SQLite database | 16777216 bytes (16 MiB), 4096 pages |

Core additionally applies its image decode limits. SQLite rollback journals and
small identity/navigation/lock files are separate from the database page ceiling.
An active schema upgrade can additionally hold one private database copy and its
rollback journal (up to 16 MiB each), separate from the live database ceiling.
These conservative development limits are explicit admission controls, not a
release-wide aggregate disk or process memory guarantee.
