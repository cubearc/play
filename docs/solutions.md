# Private solutions

A solution preserves a selected finding and your explanation. It stays private
and unverified. **Resolved locally** records your assessment; it does not submit
an answer, unlock a stage or establish external verification.

In **Explorations**, run a saved configuration and choose an actual output port.
Select **Preserve as solution**, then **Create finding from selected output**.
Write a title and explanation, optionally mark it resolved locally, and choose
**Save private solution**. Alternatively, open **Solutions** and choose
**New finding from retained results**. A free finding requires no challenge.

The saved revision freezes the final artifact/port, its type and digest, actual
retained dependencies, successful execution references and the selected steps from
their immutable configuration revisions. Other branches remain in the exploration.
Changing a graph or running it again does not change a saved solution. Literal
inputs already saved in those configurations remain local; sensitive parameters
remain redacted and must be entered again for replay.

Use **Inspect final value** to read the original result without running an
operation. Text, bytes and structured reports retain their representations.
For a string inside a report, enable **Select or convert a string field explicitly**,
enter a JSON pointer and choose text or Base64-to-bytes. An empty pointer selects
the named output itself. Numbers and other implicit conversions are rejected.
Explicit field selections share the local candidate's 64 KiB value limit.
All conversions and hashing use the existing Core-backed result handlers.

**Revise this solution** appends a revision. It can improve an explanation while
preserving the chosen earlier evidence, including when artifacts need recovery.
**Solution revision** opens earlier snapshots. Conflicting writes preserve the
draft; **Load latest revision and keep draft** explicitly refreshes its expected
revision before another save. Switching workspace sections keeps an open draft,
and closing or reloading the browser asks before discarding it.

An optional **Choose local stage candidate** reads an already imported local
challenge. Select an existing stage candidate revision to preserve its exact
artifact and explicit conversion. Saving a solution does not create or replace
that candidate or change challenge progress. Private candidate revisions remain
unverified. Changing the final selection clears its candidate association.

## Evidence and limits

**Check solution evidence again** checks current input availability without
executing operations. Changed, missing or corrupt evidence produces explicit
gaps while the saved explanation and metadata remain readable. Final content
whose artifact or digest no longer matches is rejected. An available replay path
does not prove determinism or correctness. Legacy results remain unassigned to a
graph; known artifact dependencies are followed without inventing configurations,
parameters or execution records.

Solutions use the existing private SQLite database and filesystem result bundles.
The migration preserves existing investigations. Revisions reject updates and
deletes at the database boundary. The workspace retains at most 32 solutions and
128 total revisions, each at most 512 KiB. Titles allow 512 UTF-8 bytes and
explanations require 1–8,192 bytes. Artifact content is referenced rather than
copied into SQLite and retains the shared 64-bundle / 64 MiB budget. Evidence
checks retain the existing 64-artifact / 64 MiB bounds. Quota and failed-write
errors preserve earlier records; nothing is silently evicted.

## Local API

All endpoints use existing loopback, Host/Origin and session authorization. No
request uploads data or executes an operation:

- `GET /api/solutions` lists private solution headers.
- `PUT /api/solutions` explicitly appends a revision using the expected current
  `revision` (zero for a new UUID), title, explanation, local status and source.
- `GET /api/solutions/{id}?revision=1` reads a historical snapshot and current
  replay gaps. Omit the query to read the latest revision.
- `GET /api/solutions/{id}/final?revision=1` reads and checks its final selection.

For example, after retaining a named-port result, send this JSON with the existing
session authorization to `PUT /api/solutions`:

```json
{
  "id": "84c38b53-e17b-4ba6-9497-946cc83d301f",
  "revision": 0,
  "title": "Two exact bytes",
  "explanation": "Base64 AP8= decodes to the bytes 00 ff.",
  "local_status": "finding",
  "source": {
    "type": "result",
    "selection": {
      "result_id": "6d434f94-89f3-4fb8-a7d8-c345a0000001",
      "port": "output",
      "field": null
    },
    "candidate": null
  }
}
```

Replace both UUIDs with a fresh solution UUID and your actual retained result.
An explanation-only revision uses `source:{"type":"revision","revision":1}`
for the same solution, with its latest expected revision. The server rejects
invented evidence, unknown output ports, unsupported conversions and mismatched
candidate selections. Binary final values use explicit Base64 transport.
