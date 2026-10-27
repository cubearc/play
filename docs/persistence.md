# Local investigation state

Each authorized workspace has one investigation. Its stable UUID comes from the
host launcher's registry, which identifies a canonical source directory. Reopening
that directory reuses the same database and volumes. Moving a directory creates a
new workspace; retain the old registry/volumes for explicit recovery. Play never
silently reassigns another workspace's database or uploads host paths.

The UI restores the investigation name, workspace section, explorer store/directory/page
and selected file. Existing navigation without a section opens **Files**. Explicit
file/challenge launcher intents select their corresponding section. Navigation saves
immediately through serialized, coalesced requests. Wait for **Navigation saved
locally** before closing the browser. Conflicting tabs stop autosaving and offer
**Reload investigation** rather than replacing newer state. Missing files retain
their selection and show the existing explorer/inspector recovery message.

## Storage layout

The launcher mounts separate persistent data and artifact volumes. In Play Local:

| Path | Contents |
| --- | --- |
| `/workspace/source` | Authorized source, mounted read-only |
| `/var/lib/cube-play/imports` | Explicitly imported binary/text files |
| `/var/lib/cube-play/state/investigation.sqlite3` | Investigation metadata, sessions, notes, local progress, saved navigation and artifact references |
| `/var/lib/cube-play/artifacts` | Separate output volume for the derived-artifact capability |

For native development, `CUBE_PLAY_DATA` authorizes import storage. State defaults
to its sibling `state/` directory. `CUBE_PLAY_STATE` can explicitly select a different
absolute state directory with an existing parent. State cannot overlap either source
or imports. Unix directories must be private (`0700`), database/lock files private
(`0600`); symlinks, hard-linked database files and special files are rejected. The
runtime creates these permissions, without changing source permissions. Windows
Docker uses the same non-root Linux boundary; native development uses the host
account's directory access controls.

SQLx embeds ordered migrations from `crates/play-local/migrations`. Never edit a
released migration; add the next version. Startup checks the database application
and workspace identity before migrating. Unknown versions, changed checksums or
corrupt databases fail readiness explicitly. No automatic reset is performed.
A database created by a newer Play version requires a compatible version of Play.

SQLite uses foreign keys, WAL, full synchronous writes, a single pooled connection,
a two-second busy timeout and a 128 MiB logical database budget (4096-byte pages).
Binary payloads remain on disk. Notes are limited to 100 records, 64 KiB body and
512-byte title; local progress to 1,000 opaque challenge/stage keys; artifact
references to 4,096. Saved navigation accepts only bounded relative workspace paths.
The limits apply to this initial local foundation and fail visibly when reached.

## Writes and recovery

An exclusive owner lock prevents two local application processes from owning the
same investigation. Browser clients use optimistic revisions; stale writes receive
HTTP 409. Transactions keep failed writes from leaving partially updated records.
Each application lifetime records a session; graceful shutdown records its end,
and the next start marks previously running sessions interrupted without inventing
an end time. Browser credentials are ephemeral and never stored in SQLite.

Lab results are published as complete bundles: write and sync both files in a
private staging directory, close its handle, then rename the directory atomically.
Closing directory handles before rename or recovery cleanup also allows native
Windows to save results and remove abandoned staging directories. Source files
and previously published bundles are never replaced during this process.

Import publication orders its writes: stream to a private temporary file, sync the
bytes, reserve a pending database reference, create the destination exclusively,
sync the directory on Unix, then mark the reference ready. A disconnected client
cannot cancel an already started publication. Failed publication preserves existing
names. If final database persistence fails, the complete file and pending reference
remain for recovery; the API does not expose it as a ready artifact.

On reopening, bounded shallow metadata reconciliation adopts existing pre-database
imports, completes published pending references and retains missing/changed-size
references for recovery. Abandoned unpublished reservations are released. Temporary
files left by a terminated process remain hidden and preserved for manual inspection;
no automatic cleanup deletes source data. Artifact readiness describes file availability
and size, **not** a cryptographic integrity or forensic verification claim. Generated
operation provenance and immutable derived-result workflows belong to the Lab and
artifact capabilities.

Preserve the entire stopped state directory (including any WAL/SHM files), registry
and volumes before manual recovery. Check storage permissions/free space, restore
missing imported files in the managed directory, then reopen. Do not delete the
SQLite file to silence an error. Oversized managed directories must be reduced
explicitly before recovery; Play does not scan source directories recursively.

## Local API

All routes use the same session, exact Host/Origin and request boundaries as file
operations. There is no synchronization, telemetry or automatic upload.

| Route | Behavior |
| --- | --- |
| `GET /api/investigation` | Metadata, saved view, current session and recovery notices |
| `PUT /api/investigation` | `{ "revision": 1, "title": "Case name" }` |
| `PUT /api/investigation/view` | Revision plus typed store/path/offset/selected state |
| `GET /api/investigation/sessions` | Most recent 50 local sessions |
| `GET/PUT /api/investigation/notes` | Bounded note storage with v4 UUID/revision/title/body |
| `GET/PUT /api/investigation/progress` | Opaque keys, revision and `unstarted`, `in_progress` or `completed` local status |
| `GET /api/investigation/artifacts` | Stable imported-file references and availability |

Revision `0` creates a note/progress record; subsequent writes use the returned
revision. Navigation has its own revision, independent of investigation metadata.
The API body limit is 512 KiB with smaller field limits. Progress remains private
local state and never represents official verification. The [notebook](notebook.md) provides the note editor and named derived artifacts.
[Local challenges and private candidate association](challenges.md) share the same
database and retain their referenced Lab bundles.
