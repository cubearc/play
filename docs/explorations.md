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

Parameters and choices come from Core schemas. **Parameter form** selects an
alternative contract, such as substitution by an explicit target alphabet or a
keyword. Complex values use bounded JSON. Integers outside JavaScript's exact
numeric range use decimal strings. Bytes use explicit Base64 objects.

Literal inputs are included in the saved configuration. To investigate a file,
select an authorized asset instead: its bytes stay outside SQLite, and execution
records retain its digest. Private parameters are entered for a particular run,
redacted from provenance and requested again for replay. Private operation outputs
require explicit retention; otherwise their result references are not persisted.

## Local API

The original [Lab API](lab-api.md) remains available. Named-port clients use:

- `GET /api/lab/v2/operations`: every canonical Core contract and truthful local
  `execution.available`, adapter version and limits. Only available adapters appear
  in the operation picker. Existing image inspectors continue using their direct
  Lab API while their named-port adapters are integrated.
- `POST /api/lab/v2/execute`: schema version 2, a fresh execution UUID, exact
  operation/version, named input bindings and a parameter object.
- `GET` / `PUT /api/explorations`: list or save configurations with optimistic
  revisions. New records use revision zero; a successful save returns the next
  revision. Configuration versions are immutable.
- `GET /api/explorations/{id}` and `/executions`: reopen a configuration and its
  actual execution attempts. Completed attempts are immutable; interrupted runs
  are recovered explicitly on restart.

For example, in an authenticated browser session:

```js
const response = await fetch('/api/lab/v2/execute', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
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

Legacy result bundles and notes keep their original identifiers. Their history is
unassigned to an exploration, and missing or redacted replay parameters remain
unknown rather than being inferred from terminal text.
