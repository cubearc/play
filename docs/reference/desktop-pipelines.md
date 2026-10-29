# Saved Pipelines and Trail configuration

A native Pipeline revision freezes a reusable operation definition. A Trail owns
its hypothesis, exact source Asset binding and selected Pipeline revision inside
one Exploration. [Explorations and Trails](../explorations.md#save-and-reuse-a-pipeline)
shows the editing workflow.

The development editor supports one node, with a declared `source` input. Its
operation name, installed version, input type and parameter contract come from
Core. General graphs, node connections, joins and scheduling remain tracked in
[issue 42](https://github.com/cubearc/play/issues/42) and
[issue 45](https://github.com/cubearc/play/issues/45).

## Closed commands

All mutations carry an intent `key`. Workspace operations require
`workspace_id`; Trail operations require the full owning `context`.

| Command | Additional fields | Result |
| --- | --- | --- |
| `create_pipeline` | `title`, `operation`, `operation_version`, `parameters` | `pipelineId`, `revision: 1` |
| `revise_pipeline` | `pipeline_id`, `expected_revision`, `operation`, `operation_version`, `parameters` | The same ID and a new revision, or the current revision for an identical definition. |
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

Definition format 2 contains exactly one declared input and one stable node UUID,
operation ID/version, source-slot binding and complete effective parameters.
Revising a Pipeline preserves its node UUID. A fork preserves the chosen
configuration and records an immutable Workspace-scoped origin. Definitions do
not embed Exploration Asset IDs, hypotheses, output values or secrets.

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
| Retained revisions per Pipeline | 32, including historical revisions |
| Serialized parameter object | 2048 UTF-8 bytes |
| Trail hypothesis | 2048 UTF-8 bytes |
| Pipeline title | 120 UTF-8 bytes, nonempty |

Hypotheses count toward the 4 MiB Exploration payload ceiling. Definitions remain
subject to SQLite's 16 MiB page ceiling. Reaching a limit rejects the mutation;
finished evidence is never evicted to admit work.

[Schema 9](desktop-storage.md#single-baseline-and-refusal) appends complete working
revisions for recognized earlier native definitions. It preserves their IDs,
immutable historical definition rows and accepted Run plans, outputs and input
references. Historical format-1 rows are evidence only; the editor and executor
accept complete format-2 definitions. There is no older command-shape fallback.
