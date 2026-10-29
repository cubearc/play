# Native PipelineRuns

Running a text or image cell creates a durable, Exploration-owned PipelineRun. Its saved
result and exact input evidence reopen in the notebook and its Trail canvas.
The executor plans the explicitly selected target node and its ancestors in a typed graph,
using native ROT13, Brainfuck or image pixel-reading adapters. One sandboxed worker
executes the plan sequentially. Explicit byte-representation
conversions, parallelism, executor output reuse and contextual CLI execution are
separate deliveries.

## Accepted invocation

The trusted desktop sends a closed `start_run` intent with Workspace,
Exploration, Stage, Trail ID/revision, selected Pipeline ID/revision, explicit
`target_node_id`, expected source Asset ID/revision and target operation/version,
intent UUID and explicit output-retention choice. Rust verifies every owner,
resolves the Trail's exact named Asset revisions and validates inputs/parameters against
the actual Core contract before admitting work. Unsupported native enforcement,
missing adapters, version mismatches and busy capacity reject before admission.
A changed Trail revision, source or Pipeline returns `state_conflict` before creating a Run;
the engine never silently resolves a different revision than the caller selected.
An archived source or additional bound Asset returns `block_archived` before a new Run is created. Archive
state is resolved in the engine, independently of disabled UI controls.

An accepted Run pins:

- Its engine-generated UUID, owning context, caller and intent digest.
- The full immutable Pipeline definition/revision, actual target and ordered ancestor set.
- Every named Asset revision, a `used` flag and explicit literals/connections. Only
  consumed inputs cross the private worker pipe; unused references remain in the
  saved plan and `run_inputs` for provenance. Brainfuck's
  input stream is a literal byte array, empty by default. Image reads
  bind `image.samples` from the exact authorized PNG revision, with the coordinates
  and palette-expansion choice frozen in the selected definition.
- Core operation/schema/adapter identity, complete contract and contract digest.
- Effective parameters, Trail hypothesis/revision, execution ceilings and the `none` reuse policy.

[Save and select a Pipeline revision](../explorations.md#save-and-reuse-a-pipeline)
before executing another configuration. A Run never creates or substitutes a
Pipeline implicitly. A requested operation/version that differs from the selected
target returns `pipeline_mismatch`. Reusing a definition is configuration
reuse: outputs execute again and are never presented as reused attempts.

The intent UUID identifies one accepted action. A matching retry returns the
original Run in its recorded state, including after restart. Changing its payload
returns `idempotency_conflict`. Choosing Run again after acknowledgement uses a
new intent and preserves the preceding evidence.

**Run cell** uses the saved default output. In a multi-node canvas, selecting a
node and choosing **Run node** executes that node's ancestor closure without
saving another Pipeline revision. Both actions use the same admission and
retention rules. Aggregate consumed text is bounded at 8192 UTF-8 bytes and raw
PNG data at 131072 bytes across all named inputs; extra slots do not increase
worker CPU, wall-time, memory or output budgets.

## State and recovery

Archiving a Block does not cancel admitted work, change its pinned inputs or
rewrite its evidence. Its existing Run stays supervised and cancellable. Matching
accepted-intent retries and scoped Run history remain available; new work requires
restoring the Block first.

The engine commits `queued` before spawning the worker, then records `running` and each actual
node attempt on worker progress. Completed node results commit before the next
node can start. A failed/cancelled worker preserves prior completed attempts;
unstarted consumers have no fabricated attempt. It owns supervision independently of React and
commits terminal evidence before a client can observe success.

| State | Meaning |
| --- | --- |
| `queued` | Accepted and saved; no actual node attempt has started. |
| `running` | The owned worker started and its attempt was recorded. |
| `completed` | Core output validated against the pinned named output contract. |
| `failed` | Dispatch, worker, contract or resource failure; the code explains why. |
| `cancelled` | The supervisor killed/reaped the worker and committed cancellation. |
| `interrupted` | A new owner recovered unfinished work after the previous engine died. |

A dispatch failure may have no actual attempt. A worker failure retains its real
attempt; no consumer or output is fabricated. Finished Runs, attempts, input
bindings and frozen plans are protected against mutation or deletion. Application
code and SQLite guards enforce state transitions.

Closing a canvas or application window leaves engine supervision active while
the app remains alive. Explicit Quit closes its private session: the engine
kills/reaps the active worker and saves `cancelled` with `session_closed` before
releasing SQLite ownership. A killed engine cannot report success; reopening its
recognized store marks queued/running work `interrupted` with `owner_stopped`,
without changing completed records. This recovery is not a complete release
claim about native descendant/PTY cleanup on every platform.

## Edited inputs

Saving a text/caption edit advances the working Block/Trail source, without
changing an accepted Run's plan, input references, attempts or output. An active
worker continues with its frozen bytes. The notebook/canvas marks a selected Run
from an earlier input revision; another explicit Run uses the saved revision.

A retry of an already accepted intent still recovers that original Run after an
edit. A new intent requesting the old source is rejected. The client preserves
unconfirmed intents for recovery and clears one only after a definite pre-admission
conflict or acknowledgement; it never fabricates a replacement execution.

## Outputs and privacy

**Keep result in private history** controls output retention for each Run. The
engine snapshots Core's contract and refuses unsupported sensitive-input,
sensitive-output or sensitive-parameter policies in this adapter slice. Inputs
remain references to already-authorized private Assets, rather than copied
command text or paths. There is no automatic terminal or remote capture.

Retained outputs use Core's named ports and representations: ROT13's `output`
is text; Brainfuck's `output` is the Core bytes object with Base64 representation.
The engine validates that payload against Core before atomically committing it
with the actual node attempt terminal state. The final result belongs to the
selected output node; the evidence view also shows retained intermediate outputs. The UI labels bytes as Base64 instead of guessing
a text conversion.

Without retention, every completed node and the Run record has `output_not_retained` as an explicit
replay gap, and contains neither output bytes nor their fingerprint. It cannot
silently recover discarded output. These records are private local evidence,
not Candidates, Solutions, official Challenge validation or a general replay
verifier.

## Save a selected output

Use [Save as block](../notebook.md#save-a-result-as-a-block) on a completed,
retained output to create its own notebook Asset and Trail. The Block keeps a
Run/actual-attempt/port origin; opening its source selects that exact historical
Run even after the working input changes. The immutable Run remains unchanged.

Text results can bind another saved Pipeline through their own Trail. That Run still
uses `reusePolicy: none` and records its own actual attempt, with the saved result
Asset as input. Byte results preserve exact bytes in an inspection-only File
representation. Saving a Block never fabricates an execution, recovers an
unretained output or performs an implicit conversion.

## History and limits

History queries require the exact owning context and Trail. Pages contain at most
8 summaries, ordered by the engine sequence, with a `before` cursor. Details load
one authorized Run and its evidence; another Exploration's IDs fail explicitly.
The UI can select a preceding Run and load older pages without a global Output
panel.

The shared [protocol limits](https://github.com/cubearc/play/blob/main/desktop/protocol.json)
allow 64 Runs per Exploration and 128 per Workspace. Completed evidence is never
automatically evicted to admit more work. Run snapshots, per-node results and pending
reservations count toward the 4 MiB Exploration payload ceiling; SQLite retains
its 16 MiB page ceiling. Admission reserves 128 KiB in the same transaction for
bounded terminal output, plus a 16 KiB plan allowance. Committed node results consume that reservation; finalization releases its
remainder. The aggregate raw node outputs are bounded by the existing
16384-byte Run output ceiling. Wall time (2 seconds), CPU (1 second) and VM growth
(128 MiB) apply to the one worker for the whole Run; adding nodes never multiplies
those budgets. Storage failure never becomes a successful Run.

The [native storage reference](desktop-storage.md) owns schema/upgrade and
filesystem rules. The [IPC reference](desktop-runtime.md) owns sender authority,
wire framing and worker resource ceilings.
