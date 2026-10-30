# Local Challenge package format

A package is one UTF-8 JSON file with embedded content. The engine imports it
atomically into a new Exploration using the same Stage, Block, Asset and Note
creation rules as manual authoring. See [Import a local Challenge](../challenges.md)
for the workflow and a downloadable example.

## Envelope and identities

```json
{
  "format": "cubearc.play.challenge",
  "version": 1,
  "id": "6af86a89-d708-460f-b17e-ec361190f423",
  "revision": 1,
  "title": "A hidden message",
  "origin": null,
  "stages": [{
    "id": "d4d9f976-6151-4ab5-aae9-a57bd501a7b2",
    "title": "Read the clue",
    "blocks": [{
      "id": "53cb73b1-d47d-4811-aba1-a864808550be",
      "title": "Ciphertext",
      "content": { "kind": "text", "text": "Uryyb" }
    }]
  }]
}
```

`version` identifies this format, while `revision` is a positive 32-bit package
revision. Neither establishes a published Challenge edition. Package, Stage and
Block IDs are nonzero UUIDs and must be distinct throughout the package. Local
IDs are newly allocated; stored provenance maps source Stage/Block IDs to their
local counterparts. Titles trim surrounding whitespace and require 1–120 UTF-8 bytes.

Optional `origin` is `null` or an object containing exactly `service_id`,
`challenge_id` and `edition_id`, each a nonzero UUID. These are unverified source
claims, independent of the local path, package version and package revision.
No URL, credentials or service authorization is accepted here.

## Ordered content

Stages and Blocks retain their array order. Each Stage also receives its own
empty, editable **Reasoning** block after the imported content.

| `content.kind` | Required content fields | Local representation |
| --- | --- | --- |
| `text` | `text` | Editable text Asset |
| `page` | `text` | Editable inert Page source; HTML stays text |
| `annotation` | `text` | Editable annotation Note |
| `image` | `base64`, `sha256`, `caption` | Bounded PNG Asset with grouped caption; an empty caption creates no caption Asset |
| `file` | `base64`, `sha256` | Exact file bytes with a bounded text/hex preview |

Binary content uses standard padded Base64. `sha256` is the lowercase SHA-256 of
the decoded bytes; a mismatch rejects the entire import. The package's own
recorded SHA-256 covers the exact source file bytes, including whitespace.
Integrity hashes do not authenticate an author or validate a solution.

All objects reject unknown fields and the JSON parser rejects duplicate keys.
Paths, URLs, external file references, Pipeline definitions and executable actions
are unsupported. Unsupported content types and older
`instructions`/`steps`/`hints`/`artifacts` manifests are rejected; the native
importer has no converter or compatibility reader.

## Limits and persistence

| Boundary | Limit |
| --- | --- |
| Complete source JSON file | 131072 bytes (128 KiB), including embedded Base64 |
| JSON nesting | 16 levels |
| Stages | 1–16 |
| Imported Blocks per Stage | 0–7, leaving room for Reasoning |
| Text, Page or caption | 1024 UTF-8 bytes each |
| Annotation | 2048 UTF-8 bytes |
| Decoded PNG or file | 131072 bytes each; the smaller complete-package budget still applies |

Ordinary [Workspace quotas](desktop-storage.md#development-ceilings) also apply. Existing
content is never evicted to make room for an import. Native IPC transfers only
bounded Base64 content selected by Electron's file chooser; renderer code cannot
supply host paths or package bytes through its public bridge.

Content, provenance, navigation selection and the action receipt commit together.
A validation, integrity or storage failure rolls back all of them. Retrying the
same accepted action key returns its original result; using that key with changed
content fails. A separate import creates a separate Exploration. Editing imported
content creates normal local revisions without rewriting the immutable import
provenance. No Run or node attempt is created by importing.
