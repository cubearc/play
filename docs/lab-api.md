# Typed local Lab API

The Lab calls the versioned Rust Core directly. It never parses terminal output or
executes scripts. These endpoints require the same local session cookie and exact
Host/Origin as other workspace APIs. Files/results stay local; no network operation,
telemetry or upload is added. See [the visual Lab](visual-lab.md) for interactive controls.

## Discover and execute

`GET /api/lab/operations` returns the actual Core catalog, engine/package versions,
Lab limits and execution availability. `GET /api/lab/operations/{id}` describes one
exact ID. `core.pipeline` describes Core's own adapter availability; `lab` is null
when Play cannot execute that operation. Descriptions, types, parameter constraints
and defaults come from Core. Direct image adapters work even when `core.pipeline`
is null. Masked text transforms remain unavailable because their mandatory retained
audit artifacts are outside this initial contract.

`POST /api/lab/execute` accepts non-executable JSON. Generate a fresh UUID v4 for
each request. For example, with an authenticated browser session:

```js
const response = await fetch('/api/lab/execute', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    id: crypto.randomUUID(),
    operation: 'encoding.base64.decode',
    version: 1,
    input: { type: 'text', data: 'AP+A' },
    parameters: [],
  }),
});
const result = await response.json();
// result.inline is { type: 'bytes', base64: 'AP+A' }: exact 00 ff 80 bytes.
```

Choose the adapter version advertised by `lab.version`; it is separate from the
package version, Core engine API and Play adapter API. Parameters are an array of
`{name, value}` bindings, so duplicate names are rejected. Values use Core's typed
parameter wire contract: integers, booleans, text, `{base64: '...'}`, or tagged
`{unsigned_list: [...]}` / `{text_list: [...]}`. Floats, unknown fields, duplicate
JSON keys, unknown/missing parameters and invalid values fail explicitly. Defaults
are resolved from Core descriptors and recorded as effective parameters.

Input choices are explicit:

| Input | Behavior |
| --- | --- |
| `{type: 'text', data: 'hello'}` | UTF-8 text; no inferred byte conversion |
| `{type: 'bytes', base64: 'AP+A'}` | Exact bytes; no inferred UTF-8 conversion |
| `{type: 'file', store: 'source', path: 'sample.png', representation: 'image'}` | Authorized relative file; Core decodes it as an image |
| `{type: 'artifact', id: '…'}` | Previously generated result in this workspace |

File stores are `source` and `imports`; file representations are `bytes`, `text`
or `image`. Text decoding is strict. Source/import paths are never writable;
traversal, symlinks and special files are rejected. Imported references must be
complete in the existing persistence layer. Only `image.compare` accepts and
requires an `other` input. Decode bytes with `image.decode` before using them as an
image result. Wrong semantic types need explicit conversions.

Initial direct adapters cover decode/PNG encoding, palette expansion, pixels and
channel extraction, histograms, comparison and basic exact image transforms.
Core owns every calculation. Channel results include a Core-generated grayscale
PNG preview; comparison can include an explicitly requested PNG mask. PNG result
encoding is a representation for transport; it does not preserve every source
metadata chunk or imply additional forensic precision. Image `result.metadata`
contains Core decoded-raster information: source format/origin describe decoding,
while the stored image content is PNG.

## Results and provenance

Pixel, histogram and comparison probes may set `retain_result: false`. They use
exactly the same Core worker, authorization and processing limits, return
`retained: false`, a null `content_url` and bounded inline content, and do not create
an artifact or consume its quota. Their IDs cannot be reopened or reused as inputs.
The default remains `true`; other operations require retention. Successful retained
responses include `retained: true`. Inspecting pixels must not exhaust persistent storage.

Each retained response contains a `result` record and an authenticated relative
`content_url`. Values up to 16 KiB also appear in `inline`, explicitly tagged as
text, bytes or structured JSON. Larger values use the result reference. Image
content is a Core-encoded PNG, structured content is JSON and binary content is
exact octets. `GET /api/lab/results/{id}` reopens the record;
`GET /api/lab/results/{id}/content` reads its bounded content.

Records include operation ID, package/engine/adapter contracts, effective parameters,
input references and content digests, output type/size/digest and creation time.
Pipeline parameter values are conservatively redacted, including keys and shifts;
image coordinates/channel/transform settings remain explicit. Input payloads are
not copied into provenance. Core computes SHA-256. Content integrity and workspace
identity are checked before a result can be reused.

Bundles live under `artifacts/lab/<uuid>/` on the existing persistent artifact
volume: private `record.json` metadata and raw `value.bin` content. Large binary
values never enter SQLite. Publication uses a completed-directory rename; failed
writes clean their temporary bundle. On reopen, only recognized unfinished bundles
are removed; completed inconsistent results fail clearly and are preserved.
Results survive restarts. Named/saved history and UI artifact management remain
separate from these bounded execution references.

## Execution and limits

One operation runs at a time per workspace. Core execution happens in a short-lived
`play-local --lab-worker` subprocess with cleared environment and bounded stdio;
the async API remains responsive. It accepts typed bytes, never executable paths
or filesystem roots. Existing capability-free Core pipeline adapters run one step
with Core's own value/work checks. Direct image methods retain Core's dimension,
pixel, sample, palette and result-count limits.

| Resource | Limit |
| --- | --- |
| Request JSON | Core's 256 KiB / 16 nesting levels / 4,096 nodes; five seconds to receive |
| Inline input / authorized file input | 64 KiB / 4 MiB per input |
| Output | 4 MiB; caller may lower `max_output_bytes` |
| Worker wall / CPU time | Five seconds / four CPU seconds on Unix |
| Worker address space | 128 MiB in Linux runtime; container also retains its memory limit |
| Retained payloads | 64 results / 64 MiB, plus bounded metadata |
| Concurrent content/file reads | Two, shared with workspace previews; bounded two-second acquisition wait |

Conservative Core pipeline output/work estimates can reject a request before its
actual output would reach the byte limit. Native development retains byte/work,
process and wall-clock boundaries; platform memory controls differ. The supported
ordinary-user runtime uses Linux containers for its hard address-space boundary.

Set `timeout_ms` between 1 and 5,000 to lower worker execution time. Send
`DELETE /api/lab/executions/{request-id}` to cancel active work. Disconnecting also
requests cancellation when the HTTP handler is dropped. The supervisor kills and
reaps the worker before releasing its execution slot; cancellation before
publication begins does not create a successful result. Once publication starts,
cancellation returns `commit_started`; read the completed result using the request
ID. A disconnect during that final phase may still leave a complete local result. File reads remain subject to
local filesystem availability. Operation errors use `{schema_version: 1, error: {code,
message, operation, parameter, offset}}`, preserving Core categories and safe
context without echoing arbitrary input or secret values. No unsafe fallback or
automatic retry is performed.
