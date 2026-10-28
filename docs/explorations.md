# Typed explorations

Open a workspace with `cube play`. Select **Explorations** in the workspace menu, then **New exploration**,
name the investigation and write a hypothesis. Add an operation, fill its named
inputs and parameters, then **Save configuration** and **Run selected path**.
Switch between **Flow** and **Steps** to navigate the same configuration.

Input menus offer compatible named outputs from other steps. Run the selected
path to execute missing upstream steps for this configuration. Existing successful
outputs can be reused; changing the configuration preserves earlier executions
and marks them as belonging to an earlier configuration. Opening a workspace never
runs its graph. A challenge is optional.

Use **Files** to choose an asset, then **Explore selected file** to carry that
selection into the operation forms. Switching workspace sections preserves open
drafts, results and terminal sessions. The last section and file navigation resume
after restart. **Notes**, **Artifacts**, **Local challenges** and **Terminal** each
have their own section. Single-operation text/byte utilities remain under
**Quick text and byte tools** in Explorations.

Open **Execution history**, then **View retained output** to inspect an earlier
run without executing it again. The stored artifact identity, hash and named-port
schemas must match the immutable execution evidence. Missing or private unretained
outputs remain visible as history; reopening never replaces them or runs a repair.

**Annotate selected output** carries that actual execution and named port into
Notes. Create or associate a note explicitly; an existing draft stays intact.
History also offers **Annotate execution** for observations about successful or
failed attempts. A linked note can inspect the original output after subsequent
graph edits, runs and restarts. See [private notes](notebook.md).

**Preserve as solution** carries a retained output into **Solutions**. Write an
explanation to freeze its actual dependency path in an immutable private revision
while continuing the exploration. See [private solutions](solutions.md) for
revision history, explicit field conversions and optional local stage association.

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
For these operations, **Keep this private output for this run** starts unchecked
and resets after every attempt or step selection. Keeping the output locally
enables chaining and private solutions; it never retains private parameters.
Connected private-output steps must be run explicitly before their outputs can be
used downstream. The application never retains such an upstream result implicitly.

## Reuse named outputs and structured fields

For each named input, **Load saved inputs** lists the named ports of retained typed
artifacts in this workspace. Select a compatible whole output, or explicitly select
a string field with **Field conversion** and **Field JSON Pointer**. Pointers are
relative to that named output: `/extracted`, `/items/0/name`, or `/samples/base64`.
Use `~1` for a literal slash and `~0` for a tilde. Text preserves the selected string
exactly; Base64-byte conversion uses Core and rejects invalid data. Numeric coercion,
expressions and implicit UTF-8 decoding are unavailable.

The same inputs work in Flow and keyboard-accessible Steps. Cycles are rejected,
connections to descendants are omitted, and required values remain visible. Save
configuration changes before running. A selected path displays queued/running and
final states, has at most 64 steps and a 60-second total deadline, and retains
completed evidence if a later step fails or is cancelled. A field's actual type and
the consumer's complete contract are checked before its operation executes.
Selections and exact projected-input digests persist in provenance and replay checks.

Supported CLI captures can be published explicitly in Terminal or imported from a
selected authorized source/import file with **Import selected CLI capture**. Inspect
the typed output, save it with an artifact name, then choose it with **Load saved
inputs**. Publication never reruns a command or treats a file as execution attestation;
missing original inputs and unverified CLI provenance remain visible replay gaps.

Under **Investigation scope**, keep a free exploration or use **Load local stages**
to associate a stage from the active, already imported local challenge. Switch the
active challenge in Local challenges to choose another. Save the configuration to
preserve this association. Loading/choosing a stage does not import anything, run
operations, update progress or submit a solution; existing associations survive
restart even when another challenge is active.

## Local API

See the [typed exploration API](reference/explorations.md#local-api) for request shapes and immutable execution records.

## Boundaries and recovery

Configurations and execution attempts retain their own revisions. Missing inputs, changed contracts or unavailable evidence remain explicit gaps. See [compatibility, quotas and recovery](reference/explorations.md#boundaries-and-recovery).

## Classical analysis

Select a `cryptanalysis` operation and enter its typed JSON `request` input. Models
and candidate lists are supplied explicitly and remain local. Reports show scores,
provenance and completion status; an incomplete report displays a visible notice.
Scores are heuristics, not probabilities of correctness. Candidate text is inert.
The existing Cancel action terminates the worker and never publishes an invented
completed report. The Core contract describes each request and its resource limits.
