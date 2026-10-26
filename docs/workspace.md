# Browse and inspect local files

Open a directory with `cube play [path]`. **Source files** navigates its read-only
contents; **Imported files** shows copies explicitly saved locally. Navigation is
shallow. Direct-file launches retain siblings. Exactly one compatible file in a
complete listing opens automatically; incomplete listings never claim uniqueness.

Select a file for a preview. Core validates supported PNG/JPEG/GIF/BMP/WebP rasters
before their original bytes are served. Rejected images retain metadata and byte
inspection. MP4/MOV/WebM container metadata also comes from Core. Audio/video
playback depends on browser codecs and starts muted; enable sound in the controls.
Unsupported playback retains available metadata and hexadecimal/ASCII inspection.

Filename categories are extension hints. Core header signatures are separate
evidence, never proof of structural validity. Unknown files remain selectable.
HTML, SVG and scripts are inert UTF-8 text, never embedded documents. Unsupported
text/control bytes use byte inspection without silently converting original data.

## Import explicitly

Choose a file, review **Save as**, then select **Import into workspace**. Selection
alone sends no bytes. Imports go into private data-volume storage, never the source.
Existing names are rejected; choose another name rather than overwriting them.
Imports survive restart/recreation with stable SQLite references. See
[local persistence](persistence.md) for saved navigation and missing-file recovery.

A private temporary file is flushed and linked into its final name exclusively.
Interrupted streams remove temporary files; publication already started finishes
independently of the browser request. Process crashes may leave hidden temporary
files for manual recovery. Partial imports are unavailable until reconciled. Canceling
after the commit point may leave a completed file: refresh before retrying. No
normal operation deletes completed imports. Atomic commit requires filesystem
hard-link support; unsupported storage fails explicitly.

## Limits and recovery

| Resource | Limit |
| --- | --- |
| Discovery | One level, at most 4,096 entries, sorted within that bounded set |
| Listing | 128 entries/page; no recursive scan or content decoding |
| Relative path | 4,096 UTF-8 bytes, 32 components, 255 bytes/component |
| Text | First 64 KiB, ending on a valid UTF-8 boundary |
| Bytes | 4,096 bytes/page, with absolute offsets |
| Images | 16 MiB encoded; Core additionally enforces its 4 Mi pixel/sample/dimension limits |
| Audio/video | 256 MiB, 64 KiB streaming buffers, one checked HTTP byte range |
| Imports | 32 MiB, 30 seconds |

Host changes can alter listings between pages. Refresh after renames/removals.
Truncated discovery is labeled; narrow the directory to investigate the rest.
No indexing or background analysis occurs. Filesystem latency depends on the host;
blocking work occupies at most two workers. Queued requests return retry guidance
after two seconds. Canceling a response releases streaming resources.

Roots use directory capabilities from patched `cap-std` (4.0.3 or newer; see the
[upstream confinement fix](https://github.com/sunfishcode/cap-std/security/advisories/GHSA-hp8f-xmx4-4qrg)).
Requests use relative paths through these handles rather than ambient path joins.
Symlinks are shown but not opened. Traversal, Windows devices/alternate streams,
sockets, FIFOs and other special files are rejected. Nonportable/control-bearing
names remain visible but unavailable; rename them externally if needed. Unix opens
are nonblocking against FIFO replacement races. An authorized open handle stays
authorized if the host later renames it; authorization does not revoke existing
handles. All analysis calls Core reader APIs on those authorized handles.

Authenticated routes under `/api/workspace` are `/files`, `/inspect`, `/media` and
`/import`. Session, loopback, Host/Origin, no-store, nosniff and content security
boundaries apply. No host absolute paths are returned. Only supported media MIME
types are served inline; HTML/SVG cannot become executable same-origin documents.

For the native persisted UI, set `CUBE_PLAY_DATA` to a private directory with an
existing parent, separate from the source. The launcher configures it automatically. Unsafe/missing
storage fails without falling back to source writes. Challenge data remains inert
until the dedicated challenge importer is implemented.
