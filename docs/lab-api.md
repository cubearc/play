# Native command reference

<a id="caller-owned-puzzle-profiles"></a>
<a id="deadlines-and-cancellation"></a>
<a id="discover-and-execute"></a>
<a id="errors"></a>
<a id="execution-and-limits"></a>
<a id="results-and-provenance"></a>
<a id="typed-local-lab-api"></a>

The Electron app uses a narrow typed preload and private Rust engine protocol.
It does not serve a local REST API for the notebook. Renderer code cannot access
SQLite, Node filesystem APIs or an unrestricted command channel.

| Contract | Canonical reference |
| --- | --- |
| Domain ownership and immutable evidence | [Desktop domain](reference/desktop-domain.md) |
| Trusted frames, commands, wire protocol and worker policies | [IPC and native execution](reference/desktop-runtime.md) |
| Managed stores, revisions, limits and refusal | [Native storage](reference/desktop-storage.md) |
| Saved definitions, Trail hypotheses and revision selection | [Saved Pipelines](reference/desktop-pipelines.md) |
| Accepted single-node invocations, attempts and output retention | [PipelineRuns](reference/desktop-runs.md) |

Native adapters currently execute ROT13, Brainfuck and image pixel reads on macOS. Their metadata
and schemas come from Core; catalog discovery does not authorize an adapter.
For a working example, use [your first notebook](getting-started.md).
