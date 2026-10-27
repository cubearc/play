# Typed explorations

Open a workspace with `cube play`. In **Explorations**, choose **New exploration**,
name the investigation and write a hypothesis. Add an operation, fill its named
inputs and parameters, then **Save configuration** and **Run selected path**.
Switch between **Flow** and **Steps** to navigate the same configuration.

Input menus offer compatible named outputs from other steps. Run the selected
path to execute missing upstream steps for this configuration. Existing successful
outputs can be reused; changing the configuration preserves earlier executions
and marks them as belonging to an earlier configuration. Opening a workspace never
runs its graph. A challenge is optional.

Open **Execution history**, then **View retained output** to inspect an earlier
run without executing it again. The stored artifact identity, hash and named-port
schemas must match the immutable execution evidence. Missing or private unretained
outputs remain visible as history; reopening never replaces them or runs a repair.

**Replay inputs** checks the retained dependency path when a typed result opens.
It compares current authorized file bytes and named artifact values with their
recorded digests, checks operation/schema identities and reports private parameters
or unretained literals. Changed, missing or corrupt sources remain explicit gaps.
**Check replay inputs again** repeats the check without executing an operation or
copying originals. Available inputs do not establish determinism, an identical
rerun or a correct solution. Historical configurations remain readable when their
operation versions are unavailable; saving/running still requires current contracts.

Use **Output port** to select a named result. Text remains inert, bytes retain
explicit Base64 transport, and known histogram results offer a chart and semantic
table. Other reports use a structured fallback; resembling a histogram does not
grant a chart. Previews show at most 65,536 characters, and reading a retained
bundle is limited to 4 MiB. **Output data and provenance** exposes the recorded
parameters, source references and schema digest. Name or explicitly export the
complete retained bundle using the existing artifact controls.

For an image workflow, select a source file, add `image.decode` with **Selected
file**, then connect its `output` to `image.histogram`, `image.channel.extract`
or a transform. Channel extraction keeps its report, exact `data` bytes and
grayscale `image` separate. Comparison keeps an optional `mask` as its own port.
The raster viewer displays native 8-bit samples, keyboard coordinates and zoom;
its table reads Core samples rather than browser-rendered colors. Unsupported
shapes retain their bounded structured data. Direct selected-image inputs are
decoded only in the resource-limited worker, after filesystem authorization.
Input digests explicitly identify their representation: `file_bytes` hashes the
authorized original file, while `json_wire_value` hashes a literal or named-port
value's serialized transport. These hashes are not interchangeable.

Parameters and choices come from Core schemas. **Parameter form** selects an
alternative contract, such as substitution by an explicit target alphabet or a
keyword. Complex values use bounded JSON. Integers outside JavaScript's exact
numeric range use decimal strings. Bytes use explicit Base64 objects.
Use **Use empty text** to supply an intentional empty string. Clearing an existing
text value also preserves an explicit empty string; an unset field stays absent.

For text investigations, use `text.normalize` with an explicit Unicode form,
`text.extract` with a scalar-index list or stride, and `text.statistics` for Core
counts. `text.unicode.compare` accepts separate `input` and `other` ports and
preserves both original and normalized observations. Parameter/type errors identify
the selected operation and the rejected input or parameter section.

Literal inputs are included in the saved configuration. To investigate a file,
select an authorized asset instead: its bytes stay outside SQLite, and execution
records retain its digest. Private parameters are entered for a particular run,
redacted from provenance and requested again for replay. Private operation outputs
require explicit retention; otherwise their result references are not persisted.

## Local API

The original [Lab API](lab-api.md) remains available. Named-port clients use:

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
