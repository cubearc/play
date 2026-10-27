# Resume local investigation state

The launcher keeps investigation state in private local storage. Notes, navigation,
challenge progress and named result references survive application restart.
Source files remain separate and read-only.

Resume the same directory with `cube play PATH` or choose it from **Workspaces**.
Stopping the workspace preserves its volumes. Moving the source directory creates
a different launcher path identity; keep the original state for explicit recovery.

If a source or artifact is unavailable, Play keeps readable metadata and reports
the gap. Restore the original input or choose an available result explicitly;
missing evidence is not silently replaced.

<a id="storage-layout"></a>

See [storage layout](reference/persistence.md#storage-layout) for private directories,
database paths and quotas.

<a id="writes-and-recovery"></a>

See [writes and recovery](reference/persistence.md#writes-and-recovery) before
manual storage repair.

<a id="local-api"></a>

The [local persistence API](reference/persistence.md#local-api) is contributor reference.
