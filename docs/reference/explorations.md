# Exploration API and recovery contracts

## Local API

The original [Lab API](../lab-api.md) remains available. Named-port clients use:

- `GET /api/lab/v2/operations`: every canonical Core contract and truthful local
  `execution.available`, adapter version and limits. Only available adapters appear
  in the operation picker. The original image inspectors and the named-port image
  adapters call the same actual Core APIs.
- `POST /api/lab/v2/execute`: schema version 2, a fresh execution UUID, exact
  operation/version, named input bindings and a parameter object.
- `GET` / `PUT /api/explorations`: list or save configurations with optimistic
  revisions. New records use revision zero; a successful save returns the next
  revision. Configuration versions are immutable.
- `GET /api/explorations/{id}` and `/executions`: reopen a configuration and its
  actual execution attempts. Completed attempts are immutable; interrupted runs
  are recovered explicitly on restart.
- `GET /api/explorations/executions/{id}`: inspect one actual attempt by its UUID,
  including the exploration/configuration/step reference and whether the
  editable graph has advanced. Notes may link to this exact attempt and a retained
  output port. Reading never starts a worker or repairs missing output.
- `GET /api/lab/results/{id}/replay`: inspect availability of a retained result's
  actual inputs and dependencies. Returns schema version 1, `artifact_id`,
  `status` (`inputs_available` or `gaps`), bounded check counters and typed gaps.

For example, in an authenticated browser session:

```js
const response = await fetch('/api/lab/v2/execute', {
  method: 'POST',
  credentials: 'omit',
  redirect: 'error',
  headers: {
    'Content-Type': 'application/json',
    Authorization: `Bearer ${sessionStorage.getItem('cube-play-session-capability')}`,
  },
  body: JSON.stringify({
    schema_version: 2,
    id: crypto.randomUUID(),
    operation: 'cipher.rot5',
    operation_version: 1,
    inputs: { text: { type: 'literal', value: 'A1' } },
    parameters: {},
  }),
});
const result = await response.json();
// result.inline.data is { text: 'A6' }.
```

Other bindings are `{type:'file',store:'source',path:'sample.txt',representation:'text'}`
and `{type:'artifact',id:resultId,port:'text'}`. A saved graph can also use
`{type:'step',step:upstreamStepId,port:'text'}`. Execution resolves that reference
to a real successful output from the same configuration revision. The optional
`exploration:{id,revision,step}` context must match the saved configuration; clients
cannot invent successful executions or attribute different inputs to saved steps.

Saved definitions use schema version 1, `id`, `revision`, `title`, `hypothesis`,
`scope` and `steps`. Free explorations use `scope:null`; API clients can attach an
already imported local stage with `{challenge:localChallengeKey,stage:stageId}`.
Each step has its own UUID, operation/version, named inputs, parameter object,
ephemeral `private_parameters` names and `[x,y]` position. Connections must be
acyclic and have matching semantic types. Runtime validation still checks actual
dimensions, formats, parameters and processing bounds.

## Boundaries and recovery

All endpoints require the existing session and exact Host/Origin. The existing
authorized filesystem resolver, isolated Core worker, cancellation and private
artifact bundles are reused. Contracts grant no host paths or network access.
Sources remain read-only; exporting is an explicit separate action.

Requests reject duplicate JSON keys, more than 16 nesting levels, more than 4,096
nodes or 256 KiB. Named inputs/outputs have a 4 MiB **serialized JSON** budget;
parameters have 64 KiB. These interface budgets include escaping and wrappers and
can be stricter than Core's raw input bounds. The default worker deadline is five
seconds. Discovery reports the actual interface budgets separately from Core caps.

Storage is bounded to 32 explorations, 64 steps per graph, 256 configuration
versions totaling 8 MiB, and 512 execution attempts with at most 32 KiB of evidence
each. Binary results reuse the existing 64-bundle / 64 MiB artifact budget. Quota
errors preserve previous records; no evidence is silently evicted. Failed,
cancelled and interrupted attempts retain their real status. Missing or corrupt
artifacts fail explicitly, and historical references remain intact.

Replay checks inspect at most 64 distinct retained bundles, stop at a cumulative
64 MiB reading budget (one final bounded 4 MiB read may cross that threshold), and
return at most 256 gaps. They use the existing authorized resolver and shared read
permit. They do not decode images, execute graphs or scan unrelated workspace files.

Legacy result bundles and notes keep their original identifiers. Their history is
unassigned to an exploration, and missing or redacted replay parameters remain
unknown rather than being inferred from terminal text.

The typed Core bridge also executes the bounded transposition/fractionation
profiles: exact scalar rail/column/sequence/route/grille permutations and explicit
coordinate/Morse hybrids. All 48 new operations retain canonical named ports;
none requires a pipeline adapter to execute directly. Complete Columnar keeps
`original_length`/`padding_count`, Cardan overlays retain `positions` and
`original_selected`, and Morse returns separate `text`, `ciphertext`, `morse`,
`padding_count` and `padding_ambiguous` fields. An unresolved Morse inverse has
`text: null`; the structured viewer preserves this evidence without inventing
plaintext. Parameters, source literals and artifact references are validated by
Core schemas and the existing capability-gated worker. No network access or
upload is introduced. See the pinned Core guide `docs/cipher-permutations.md`
for exact keyword ties, shapes, route directions, alphabets and pad conventions.

The local execution catalog covers every installed Core operation. Discovery is
separate from execution authorization: filesystem operations require selected
workspace files, and limits remain visible in each execution descriptor. File
metadata inspects only a bounded header, even for a large file; its
`file_metadata_json` digest identifies metadata, not complete file contents.
Carving creates a derived artifact from an explicit range or validated candidate;
it does not write back into source directories.

Brainfuck uses separate `source` text and `input` byte-array ports (operation
version 2). Profile execution also requires an explicit profile (version 2).
Earlier incompatible configurations remain readable and require deliberate edits.
Color exports expose aligned PNG `images`, QR exposes nullable byte `payloads`,
and text-grid rendering adds `image` only when an explicit complete color mapping
is supplied. Generic structured viewers preserve these results without guessing
codecs, text encodings or forensic meaning.

For operations marked with private inputs, entered values apply to the current
run. They are cleared after each attempt or step selection and never enter the
saved configuration. Authorized file and artifact references can still be saved.
Execution evidence omits private input values and fingerprints and reports a
replay gap. An unretained private output also leaves no output digest in saved
execution history. Retention remains an explicit, separate choice for each run.
