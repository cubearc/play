# Native workspace storage

The Rust engine owns each managed Workspace, its independent Explorations, local
Stages, ordered content Blocks, Assets, Trails and reusable Pipeline definitions.
The [first notebook](../getting-started.md) shows creation, reasoning and
reopening; [Asset imports](../workspace.md) covers PNG and file selection. The
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

The native store uses generation **2**, schema version **6**, SQLite application ID
`0x4355504e` and 4096-byte pages. The native baseline is in `crates/play-engine/src/workspace/schema.sql` and its
accepted generation evolves through `runs.sql`, `copies.sql`, `archive.sql`,
`files.sql` and `results.sql`.
Recognized native schemas 1–5 upgrade to schema 6 under the owner lock, preserving content,
IDs and Run evidence. The engine
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

## File content and previews

File Assets use `kind: file` independently of their extension. Main's native
chooser copies at most 131072 bytes from one regular file through a bounded,
read-only handle; special files are refused without a blocking stream read.
Only the basename enters the domain title; the source path is not stored.
The internal closed `add_file` command carries `context`, `key`, `title` and
`base64`. Rust validates ownership, strict Base64, filename/title and decoded
size, then atomically creates an immutable revision/digest, Block and inspection
Trail. The existing active/retained/aggregate byte ceilings and receipts apply.
Malformed data, missing ownership or quota rejection leaves no partial content.

Views expose `content: { kind: file, byteLength, sha256, preview }`, without the
full bytes. The engine verifies the stored digest before deriving `preview`:
`{ format: utf8 | hex, text, truncated }`. Valid UTF-8 with no controls other
than tab, CR and LF uses at most the first 256 bytes, shortened to a character
boundary. Other content uses Core's hexadecimal encoder over at most 256 bytes.
`truncated` refers to the exact original byte length; empty files are supported.
Neither extensions nor apparent markup grant HTML, scripting or network behavior.

File Trails have no executable Pipeline binding in this preview. Files are not
coerced to text inputs or editable with `revise_text`. Their Blocks use the same
ordering, independent copy/lineage and archive/restore lifecycle as other Assets;
copies retain exact bytes, while archived views remain readable. Schema 5 widens
the Asset-kind constraint while preserving existing references and triggers.

## Saved Run outputs

The closed `pin_run_output` intent carries `context`, `key`, `expected_revision`,
`run_id`, `attempt_id` and `port`. The renderer cannot supply output bytes, a
replacement value, a title or an inferred source. Rust checks the exact owned
Stage revision and completed retained Run/attempt, its single target node,
installed Core contract/version/digest and named output schema. Unsupported
privacy policies, contract drift, missing ports, failed or unretained work refuse
promotion without changing the notebook.

The transaction copies the selected text or bytes into a new immutable Asset
revision, creates its Block/Trail, records the origin and advances the Stage.
Text preserves the text semantic type and existing 1024-byte ceiling. Bytes use
the File representation and its bounded viewer, without implicit text coercion.
Active/retained Block and aggregate byte limits apply to the complete mutation.
No operation runs while saving an output.

`result_origins` links the new Asset to its exact Run, actual attempt and named
port under composite ownership foreign keys. An immutable unique key prevents
multiple roots for that selection. Guards preserve the origin, prevent further
root content revisions and exclude simultaneous copy/Run origins. Existing
`asset_origins` still records the immediate parent of an independent copy.

Protocol 9 exposes a tagged Asset origin: `kind: asset` has `assetId`, `revision`
and `title`; `kind: run_output` has `runId`, `attemptId`, `nodeId`, `trailId`,
`port`, `semanticType`, `operationName` and the engine `sequence`. There is no
untagged-origin fallback. Run-origin fields derive from immutable stored evidence,
and the source link requests that exact Run within its owning context.

Result roots reject `revise_text` with `result_read_only`; copied text variants
have their own revisions and remain editable. Reordering/archive/restoration
preserve the root's origin and history. Archiving a source clue does not revoke
its retained Run or the independent result. Text results can feed a new explicit
text Run; byte results remain inspection-only Files in this preview.

Matching intent retries return the original receipt. A fresh intent for the same
Run/attempt/port returns its existing Block without adding content or restoring
an archive. A stale Stage revision fails before mutation. `result_not_available`
identifies unfinished or unretained work. Recognized native schema 5 upgrades
preserve files, archived content, source lineage and completed Runs before
accepting promotion.

## Block ordering

Stage views return one ordered active `blocks` sequence tagged with `kind: asset` or
`kind: note`. The reasoning Note is a Block with its own ID and revisioned Note
content; there is no separate `Stage.note` field. Image and caption belong to one
Asset Block. The current baseline has exactly one reasoning Block per Stage.

The closed `reorder_blocks` intent carries `context`, `key`, `expected_revision`
and `block_ids`. Supply every active Block UUID in that Stage exactly once, including
reasoning. The engine rejects archived, omitted, duplicate, foreign or caption Asset IDs
with `invalid_order`, and stale Stage revisions with `state_conflict`. Preload
also checks UUIDs, uniqueness and the eight-Block ceiling before forwarding.

One transaction changes positions and advances the Stage revision. An identical
order is a no-op; its revision stays the same. Mutation receipts preserve accepted
responses for matching retries. The command never revises Assets, Notes, Trails,
Pipelines or Runs. A failed move changes nothing. Reopening returns the saved
sequence, independently for every Stage and Exploration.

Stage revisions track structural membership and order. Adding text, an image or a file
also advances this revision; editing content or saving reasoning uses its own
revision. Content is inserted before a trailing reasoning Block, otherwise at the
end. Positions are normalized among active Blocks; archived positions cannot
change that order. The coordinated app/engine wire protocol is **7**, with no older
DTO fallback.

## Block copies and provenance

The closed `duplicate_block` intent carries `context`, `key`, `block_id`,
`expected_revision` and `source`. `expected_revision` is the Stage's structural
revision. `source` contains `asset: { asset_id, revision }` and a matching
`caption` reference or `null`. Both source references must match the selected
Block's saved bindings. Concurrent content edits are conflicts even when the
Stage order did not change. Missing/foreign Blocks fail; Note Blocks return
`invalid_block_type`.

One transaction checks the Block and retained byte ceilings, copies the selected
content into new Asset UUIDs at revision 1, records their exact source revisions,
creates a fresh Trail for each Asset, inserts the complete Block after its source
and advances the Stage revision. Image bytes are verified and decoded by Core;
caption limits remain in force. A failure, including a caption quota rejection
after preparing the image, rolls back the entire copy and receipt.

The fresh Trail reuses the source Asset's first working Trail Pipeline binding;
it starts with an empty hypothesis and no Runs. The operation selector remains a
draft for a later explicit Run. Accepted execution history, private retained
outputs and reasoning are never cloned or run implicitly. Source IDs/revisions,
Trails, Pipelines and Runs are not rewritten. Copies use distinct bounded UTF-8
titles with numbered suffixes when needed.

`asset_origins` stores the copied Asset ID, source Asset ID and source revision
under the same Workspace/Exploration foreign keys. Update/delete triggers make
that relationship immutable. Asset views expose optional
`origin: { assetId, revision, title }`; originals omit it. Editing a copy appends
its own revision while retaining this origin. Copying a copy records the immediate
source revision, preserving the lineage chain.

Matching mutation-key retries return the original accepted Block, even after
later edits. Changed payloads with that key fail. The renderer retains an
unacknowledged intent so retrying a transport failure cannot silently create a
second copy; confirmed conflicts reload the current notebook and preserve Note
drafts. Copies count as separate content for all storage/Block ceilings. The
single reasoning Block has no copy action in this preview.

## Block archive and restoration

`archive_block` and `restore_block` carry `context`, `key`, `block_id`,
`expected_revision` and the same closed `source` snapshot as duplication.
Admission checks the exact owning Stage, structural revision and primary/caption
bindings. Concurrent content edits conflict even if the Stage order is unchanged.
Note Blocks return `invalid_block_type`.

Schema 4 adds nullable `blocks.archived_revision`. `NULL` means active; a positive
value pins the Stage revision that last archived that Block. A CHECK constraint
keeps the unique reasoning Block active. A deletion trigger preserves Block rows.
There is no permanent-delete command or implicit data cleanup.

One transaction changes the archive state, advances the Stage revision and
normalizes active positions. A request for the current state at the expected
revision is a no-op; stale revisions still conflict. Matching receipt retries
return the original accepted response and never reapply an old archive after a
later restoration. Failures leave membership, content and evidence unchanged.

Stage responses contain `archivedBlocks`, newest archive revision first. Each
record has `archivedRevision` and `block: { id, asset, caption? }`; the inner Block
is always asset content, without a Note variant. Active Blocks retain their tagged
shape. Restoring inserts the original Block before trailing reasoning, otherwise
at the active end. IDs, Asset revisions, copy provenance, Trail bindings,
Pipelines and Runs are retained. `Trail.archived` is derived from its active Block
binding rather than stored as a second mutable status.

Archived text/captions cannot be edited or duplicated. New Run admission rejects
their source in the engine; already admitted work keeps its original supervision,
completion/cancellation and history. Content/image reads and existing Run evidence
remain scoped and available. An active copy can still run when its source of
provenance is archived.

The active ceiling remains eight Blocks. Adding or duplicating also checks the
sixteen-retained-Block ceiling, returning `retained_block_limit` when reached.
Restoration checks active capacity without creating another retained row.
Archiving remains possible at the retained ceiling, so an active/archived pair can
still be exchanged. All retained revisions and Runs continue to consume byte
quotas; archiving does not reclaim that budget. Renderer controls are capacity
hints; Rust enforces the limits and ownership independently.

## Text and caption revisions

The closed `revise_text` intent carries `context`, `key`, `block_id`, `asset_id`,
`expected_revision` and `text`. The engine resolves a text Asset actually bound
to that Stage Block, checks its current and latest revision, validates UTF-8 byte
limits and charges retained bytes before appending. Image and File Assets are not editable
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
| Active Blocks per Stage, including reasoning | 8 |
| Retained Blocks per Stage, including active content, archives and reasoning | 16 |
| Text Asset or image caption | 1024 bytes |
| One reasoning revision | 2048 bytes |
| One imported PNG | 131072 bytes (128 KiB) |
| One imported file | 131072 bytes (128 KiB) |
| File preview source prefix | 256 bytes (512 hexadecimal characters for binary content) |
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
