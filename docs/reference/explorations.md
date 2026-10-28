# Exploration API and recovery contracts

## Local API

Use these routes for named-port operations and saved graphs. The
[single-input Lab API](../lab-api.md) documents its separate version-1 request
format. Named-port clients use:

- `GET /api/lab/v2/operations`: every canonical Core contract and truthful local
  `execution.available`, adapter version and limits. Only available adapters appear
  in the operation picker. Image inspectors and named-port adapters call the same Core APIs.
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

### Requests and execution

Duplicate JSON keys are rejected. These are Play interface limits in addition to
Core's operation limits:

| Resource | Maximum or default |
| --- | --- |
| Request JSON | 256 KiB |
| Nesting | 16 levels |
| JSON nodes | 4,096 |
| Named-input JSON | 4 MiB |
| Named-output JSON | 4 MiB |
| Parameter JSON | 64 KiB |
| Worker deadline | Default five seconds |

Serialized budgets include wrappers and escaping. Discovery reports these
separately from Core caps. See
[Core and interface limits](https://github.com/cubearc/cube/blob/main/docs/concepts/resource-limits.md#core-and-interface-limits).

### Retention and recovery

| Resource | Maximum |
| --- | --- |
| Explorations | 32 |
| Steps per graph | 64 |
| Configuration versions | 256, totaling 8 MiB |
| Execution attempts | 512 |
| Evidence per attempt | 32 KiB |
| Binary artifact bundles | 64, totaling 64 MiB |

Quota errors preserve previous records; evidence is not silently evicted. Failed,
cancelled and interrupted attempts retain their status. Missing or corrupt
artifacts fail explicitly, with historical references intact.

Replay checks inspect at most 64 distinct retained bundles, stop at a cumulative
64 MiB reading budget (one final bounded 4 MiB read may cross that threshold), and
return at most 256 gaps. They use the existing authorized resolver and shared read
permit. They do not decode images, execute graphs or scan unrelated workspace files.

Legacy result bundles and notes keep their original identifiers. Their history is
unassigned to an exploration, and missing or redacted replay parameters remain
unknown rather than being inferred from terminal text.

## Operation-specific results

Core's [transposition and fractionation reference](https://github.com/cubearc/cube/blob/main/docs/cipher-permutations.md)
defines keyword ties, routes, shapes, alphabets and padding. The Play viewer
preserves separate named outputs:

- Complete Columnar retains `original_length` and `padding_count`.
- Cardan overlays retain `positions` and `original_selected`.
- Morse exposes `text`, `ciphertext`, `morse`, `padding_count` and
  `padding_ambiguous`. An unresolved inverse has `text: null`.

### Books, keyboards and symbols

Book encoding has separate `source` and `text` inputs. Its `coordinates` output
can feed the decoder's `input` port when parameters contain the identical caller
source. The source hash identifies exact UTF-8 bytes and line endings.

Keyboard coordinates are `[row,column,layer]` arrays. Select the named JSON
output to reuse coordinates without text conversion. Core's contract defines
geometry, origin, displacement and edge policies. Symbol records preserve exact
aliases and byte/scalar positions; ambiguous inverses fail.

Text fields use multiline editors. Browser text editing uses LF; file-backed
source ports preserve original UTF-8, including CRLF. The hash identifies the
exact value sent to Core. The schema editor handles caller-owned tables/geometry;
viewers retain separate text, coordinate, position and provenance outputs.

[Core profile limits](https://github.com/cubearc/cube/blob/main/docs/ciphers.md#caller-owned-book-keyboard-and-symbolic-profiles)
include 64 KiB definitions, 1 MiB inputs, 65,536 items and 4 MiB reports.
Play's request and artifact quotas apply as well. Definitions and results stay
within the authorized workspace; no OS keyboard, font, remote source or OCR is
used to infer a mapping.

### Files and generated images

The local execution catalog covers every installed Core operation. Discovery is
separate from execution authorization: filesystem operations require selected
workspace files, and limits remain visible in each execution descriptor. File
metadata inspects only a bounded header, even for a large file; its
`file_metadata_json` digest identifies metadata, not complete file contents.
Carving creates a derived artifact from an explicit range or validated candidate;
it does not write back into source directories.

### Versioned configurations

Brainfuck uses separate `source` text and `input` byte-array ports (operation
version 2). Profile execution also requires an explicit profile (version 2).
Earlier incompatible configurations remain readable and require deliberate edits.
Color exports expose aligned PNG `images`, QR exposes nullable byte `payloads`,
and text-grid rendering adds `image` only when an explicit complete color mapping
is supplied. Generic structured viewers preserve these results without guessing
codecs, text encodings or forensic meaning.

## Private inputs and output retention

For operations marked with private inputs, entered values apply to the current
run. They are cleared after each attempt or step selection and never enter the
saved configuration. Authorized file and artifact references can still be saved.
Execution evidence omits private input values and fingerprints and reports a
replay gap. An unretained private output also leaves no output digest in saved
execution history. Retention remains an explicit, separate choice for each run.

Enigma uses a named `text` input with `text`, `final_windows` and `trace` outputs.
Settings, input and state results are private. Raw stream transforms likewise use
per-run private keys/pads and an explicit retention choice. Use installed Core
contracts for profile-specific ports and limits.
