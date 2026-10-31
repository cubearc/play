# Native persistence and recovery

<a id="local-api"></a>
<a id="local-investigation-state"></a>
<a id="storage-layout"></a>
<a id="writes-and-recovery"></a>

Managed Workspace data lives in private SQLite, with immutable revisions, an OS-held owner lock and explicit format refusal.

- [Storage layout, baseline and refusal](desktop-storage.md)
- [Run retention and interruption](desktop-runs.md)
- [Resume saved work](../persistence.md)
