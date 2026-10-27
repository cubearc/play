# Private notes and artifacts

Open **Notes**, then use **New note** to record a hypothesis. Notes autosave after 400 ms and serialize
edits made during an active write. Wait for **Notes saved locally** before leaving.
A note can refer to the selected source/imported file or the investigation. Notes
are plain text; HTML and scripts are inert. Stale revisions stop autosave and retain
the draft in the editor. Copy it before explicitly reloading/discarding it; newer
stored content is never silently overwritten. Empty titles must be corrected.

Select any generated Lab result, enter an artifact name and choose **Save named
artifact**. Names and optimistic revisions live in the existing SQLite database;
exact bytes, type, parameters, digests and lineage stay in immutable Lab bundles on
the artifact volume. No second database or binary copy is created. Refresh the
**Artifacts** section to reopen or rename results. **Load operation history** reads at
most 64 retained bundles; it performs no recursive source analysis. References to
missing/corrupt results remain visible for recovery. Reopened image results can
use the same visual controls; two compatible saved images can be compared by Core.

Under **Explorations**, expand **Quick text and byte tools** for single-operation
Core tools with default parameters. With
no file selected it accepts explicit private text; with a file it uses the selected
Core input type and strict text decoding or exact bytes. It does not parse terminal
output or implement encoding algorithms. Use the real terminal for other parameters.
Text previews are bounded to 64 KiB; byte previews use Base64 API transport without
implicit UTF-8 conversion. Explicit export retains all original result bytes.

**Export selected artifact** is a deliberate browser download. Its authenticated
endpoint accepts a bounded portable filename, never a host destination path. Your
browser owns destination authorization and normal collision handling; ordinary
browser downloads preserve existing names by choosing another filename. Play cannot
write through host paths, follow destination symlinks or overwrite sources. Use the
browser's save controls to choose a location; any explicit browser replacement is
a separate user decision. The server neither mounts an export directory writable
nor writes back into the source. Export errors leave bundles and database records
unchanged; an incomplete browser download can be discarded and retried.

State survives server restart and container recreation using the existing private
volumes. Names are limited to 512 UTF-8 bytes; notes to 100 records/64 KiB body;
results retain the existing 64-bundle/64-MiB quota. No files, notes, commands or
artifacts are uploaded and no telemetry is added. Preserve stopped volumes and
SQLite/WAL files for recovery as described in [persistence](persistence.md).

New endpoints use the existing local session and Host/Origin checks:
`GET/PUT /api/lab/library`, `GET /api/lab/history`, and
`GET /api/lab/results/{id}/export?name=result.bin`. Library writes use an existing
result UUID, title and revision (0 creates; subsequent writes use the returned
revision). Export names reject paths, reserved names and header injection.

Historical cipher profiles use the same typed exploration controls and named
ports. Gronsfeld, Autokey, Quagmire, square/matrix ciphers and the other explicit
profiles are implemented in Core; see the [pinned Core cipher guide](https://github.com/cubearc/cube/blob/750b4b783b9d85829080e2769c29b46dd84eb414/docs/ciphers.md).
Playfair exposes text, prepared text and filler positions separately; decode
retains fillers. Solitaire deck/passphrase parameters stay ephemeral, and
private results require explicit retention through the existing private-output
flow. These direct operations add no implicit uploads or pipeline adapters.
