# Resume saved work

<a id="local-api"></a>
<a id="resume-local-investigation-state"></a>
<a id="storage-layout"></a>
<a id="writes-and-recovery"></a>

Quit and reopen the Electron preview to restore the last successfully opened
Workspace, Exploration and Stage. The Rust engine saves text/image Blocks, Asset
identities and revisions, saved reasoning, Trails, initial Pipeline definitions
and accepted single-node Runs in private local storage.

Retained outputs reopen beside their cells and in Trail history. A Run accepted
without output retention reopens with an explicit gap. Operation-selection
drafts and open canvas tabs are not persisted. Save reasoning and wait for
**Saved** before switching; unsaved drafts do not survive a crash.

## Interrupted execution

Closing a canvas or window leaves supervision active while the app stays alive.
Explicit **Quit** closes the engine session and cancels its active worker. After
an engine crash, reopening a recognized store marks unfinished accepted Runs
**interrupted**. Completed evidence is not rewritten. See
[Run state and recovery](reference/desktop-runs.md#state-and-recovery).

## Storage and recovery

Private data lives under `CubeArc/Play` in Electron's per-user application-data
directory, outside imported source files. Do not move databases into a source
folder or delete an owner lock to force access. Another live owner produces
`workspace_busy`; an unsupported or altered store is refused without reset.

An incompatible store remains intact. Use a supporting app version or create a
separate Workspace. There is no old product-store importer or automatic reset.
Recognized native schema upgrades are described in
[native storage](reference/desktop-storage.md), together with exact paths, quotas
and lock ownership. Signed updates, backup/export and uninstall preservation have
not been delivered by this development preview.
