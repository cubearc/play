# Native PipelineRuns

Running a text cell creates a durable, Exploration-owned PipelineRun. Its saved
result and exact input evidence reopen in the notebook and its Trail canvas.
The development executor supports one node using the installed native ROT13 or
Brainfuck adapter. General DAG planning, joins, parallelism, output reuse and
contextual CLI execution are separate deliveries.

## Accepted invocation

The trusted desktop sends a closed `start_run` intent with Workspace,
Exploration, Stage, Trail, selected Pipeline revision and expected source Asset
ID/revision, operation/version,
intent UUID and explicit output-retention choice. Rust verifies every owner,
resolves the Trail's exact Asset revision and validates inputs/parameters against
the actual Core contract before admitting work. Unsupported native enforcement,
missing adapters, version mismatches and busy capacity reject before admission.
A changed Trail source or Pipeline returns `state_conflict` before creating a Run;
the engine never silently resolves a different revision than the caller selected.

An accepted Run pins:

- Its engine-generated UUID, owning context, caller and intent digest.
- The immutable Pipeline definition/revision, stable node ID and target set.
- Named input Asset revisions and any adapter-supplied literals. Brainfuck's
  input byte stream is explicitly empty in this development adapter.
- Core operation/schema/adapter identity, complete contract and contract digest.
- Effective parameters, execution ceilings and the `none` reuse policy.

Selecting another operation freezes a separate Workspace Pipeline definition;
other Trails and earlier Runs retain their chosen definitions. Matching
single-node definitions can be reused with independent input bindings. This is
configuration reuse; outputs are executed again and never presented as reused
attempts. The operation selector remains a draft for the next accepted Run.

The intent UUID identifies one accepted action. A matching retry returns the
original Run in its recorded state, including after restart. Changing its payload
returns `idempotency_conflict`. Choosing Run again after acknowledgement uses a
new intent and preserves the preceding evidence.

## State and recovery

The engine commits `queued` before spawning the worker, then records an actual
node attempt and `running`. It owns supervision independently of React and
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
with Run/attempt terminal state. The UI labels bytes as Base64 instead of guessing
a text conversion.

Without retention, the completed record has `output_not_retained` as an explicit
replay gap, and contains neither output bytes nor their fingerprint. It cannot
silently recover discarded output. These records are private local evidence,
not Candidates, Solutions, official Challenge validation or a general replay
verifier.

## History and limits

History queries require the exact owning context and Trail. Pages contain at most
8 summaries, ordered by the engine sequence, with a `before` cursor. Details load
one authorized Run and its evidence; another Exploration's IDs fail explicitly.
The UI can select a preceding Run and load older pages without a global Output
panel.

The shared [protocol limits](https://github.com/cubearc/play/blob/main/desktop/protocol.json)
allow 64 Runs per Exploration and 128 per Workspace. Completed evidence is never
automatically evicted to admit more work. Run snapshots, results and pending
reservations count toward the 4 MiB Exploration payload ceiling; SQLite retains
its 16 MiB page ceiling. Admission reserves 128 KiB in the same transaction for
bounded terminal output, plus a 16 KiB plan allowance. Finalization releases that
reservation. Storage failure never becomes a successful Run.

The [native storage reference](desktop-storage.md) owns schema/upgrade and
filesystem rules. The [IPC reference](desktop-runtime.md) owns sender authority,
wire framing and worker resource ceilings.
