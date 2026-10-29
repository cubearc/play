# Saved Pipelines and Trail configuration

A native Pipeline revision freezes a reusable operation definition. A Trail owns
its hypothesis, exact source Asset binding and selected Pipeline revision inside
one Exploration. [Explorations and Trails](../explorations.md#save-and-reuse-a-pipeline)
shows the editing workflow.

The editor supports a typed graph with one declared `source` input, up to eight
nodes and one selected output node. Native ROT13, Brainfuck and image pixel reads
use their installed Core names, versions, named ports and parameter contracts.
Rust validates the whole graph and plans the chosen output's ancestor closure;
one bounded worker executes those dependencies sequentially. Additional Asset
slots, explicit representation conversions, parallelism, output caching and
contextual CLI attachment remain separate deliveries under
[issue 42](https://github.com/cubearc/play/issues/42) and
[issue 45](https://github.com/cubearc/play/issues/45).

## Closed commands

All mutations carry an intent `key`. Workspace operations require
`workspace_id`; Trail operations require the full owning `context`.

| Command | Additional fields | Result |
| --- | --- | --- |
| `create_pipeline` | `title`, `definition` | `pipelineId`, `revision: 1` |
| `revise_pipeline` | `pipeline_id`, `expected_revision`, `definition` | The same ID and a new revision, or the current revision for an identical definition. |
| `fork_pipeline` | `pipeline_id`, `revision`, `title` | A distinct Pipeline at revision 1, with the exact source revision recorded. |
| `get_pipeline` | `pipeline_id`, `revision` | That exact selectable definition, available revisions and optional fork origin. No mutation key. |
| `save_trail` | `trail_id`, `expected_revision`, `hypothesis`, `pipeline` | `trailId` and the advanced Trail revision. |

A selected `pipeline` is `{ id, revision }` or explicit `null`. Revisions are
positive integers. A stale Pipeline head or Trail revision returns
`state_conflict`; saving a fork never edits its source. Receipts make matching
mutation retries idempotent. Reusing an intent with changed content returns
`idempotency_conflict`.

The engine rejects foreign owners, missing revisions, incompatible Asset types,
unsupported native adapters, wrong operation versions and invalid parameters
before committing anything. Archived sources cannot change Trail configuration.
Text/caption edits also advance the bound Trail revision, preventing an older
configuration editor from overwriting a newer binding.

## Definitions and execution

Definition format 3 contains `inputs: [{name: "source", semanticType}]`, a `nodes`
array and the selected `output` node UUID. Each node has a stable `id`,
`operation`, `operationVersion`, complete `parameters`, a bounded `position`
(`x`, `y`) and named `inputs`. A binding is exactly one of:

- `{kind: "input", name: "source"}` for the Trail's Asset.
- `{kind: "output", nodeId, port}` for another node's named output.
- `{kind: "literal", value}` for an explicit Core-compatible value.

The engine rejects duplicate/missing node IDs, missing/extra ports, cycles,
unknown source slots, incompatible semantic types and unsupported versions before
saving. Consumers validate resolved values against their full Core schema before
execution. Matching semantic types do not bypass stricter value constraints.
There is no implicit conversion: Brainfuck's `bytes.array` input takes a literal
JSON byte array, while pixel and Brainfuck outputs are Core `bytes` objects with
Base64 representation. Connecting those representations directly is rejected.
Text literals and byte arrays belong to the saved definition, so they must not
contain credentials or other secrets.

A fork preserves the chosen graph and records its immutable Workspace-scoped
origin. Revisions preserve node IDs unless the editor deliberately removes/adds
a node. Layout belongs to the revision. Definitions do not embed Exploration
Asset IDs, hypotheses or output values.

Trails keep their selected revisions when a Pipeline head advances. New text and
image Assets bind an existing matching default definition, or create that default
explicitly as part of the Asset transaction. A changed head cannot change the
default operation for a later Asset. Current revision reads resolve the selected
revision rather than assuming every Trail uses the latest head.

A Run requires the selected Pipeline ID/revision and source ID/revision. Its
operation/version are preconditions, not a request to create another definition.
A mismatch returns `pipeline_mismatch`. The saved plan freezes the Pipeline,
parameters and Trail hypothesis/revision at admission. Earlier Runs remain
unchanged when the working hypothesis, Pipeline or Asset changes.

The signed worker executes the saved parameters through Core. Image reads accept
`x`, `y` and `expand_palette`; coordinates are checked against the authorized image
before admission. Brainfuck accepts its three Core limit fields, each constrained
by the native adapter's existing ceiling. ROT13 accepts an empty parameter object.
The editor cannot raise worker CPU, time, memory or output ceilings.

## Limits and storage

| Limit | Maximum |
| --- | --- |
| Pipelines per Workspace | 32 |
| Selectable format-3 revisions per Pipeline | 32 |
| Nodes / serialized definition | 8 / 8192 UTF-8 bytes |
| One literal | 2048 serialized UTF-8 bytes |
| Canvas coordinates | Integers from 0 to 4096 |
| Serialized parameter object | 2048 UTF-8 bytes |
| Trail hypothesis | 2048 UTF-8 bytes |
| Pipeline title | 120 UTF-8 bytes, nonempty |

Hypotheses count toward the 4 MiB Exploration payload ceiling. Definitions remain
subject to SQLite's 16 MiB page ceiling. Reaching a limit rejects the mutation;
finished evidence is never evicted to admit work.

[Schema 10](desktop-storage.md#single-baseline-and-refusal) appends executable
graph revisions for recognized earlier native definitions, preserving selected
configurations, IDs, historical rows and accepted Run snapshots. Only format-3
definitions are selectable/executable; historical formats remain immutable
evidence. The command boundary accepts only complete `definition` payloads.
