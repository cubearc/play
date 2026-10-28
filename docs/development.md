# Develop Cube Play

Use Rust stable through rustup, Node.js 24.15+ and npm. Keep Cube 0.18.0 at `../cube`.

## Electron boundary proof

Use the pinned Rust toolchain and `frontend/.nvmrc`. On macOS, install the Xcode
Command Line Tools for the native helper build and local ad-hoc signing.

```sh
npm ci --prefix frontend
npm run desktop:build --prefix frontend
npm run desktop:test --prefix frontend
npm run desktop:smoke --prefix frontend
npm run desktop:proof --prefix frontend
```

The preview runs packaged React, Electron, Rust and Core over private pipes, with
no server or Docker. Windows/Linux execution is refused until native policy
validation is implemented. See the [IPC and native execution contract](reference/desktop-runtime.md)
for its exact scope, resource accounting and remaining release requirements.

### Explore the notebook preview

The workbench uses [VSCode Elements](https://vscode-elements.github.io/) with typed
React components, Codicons and VS Code theme variables. The resizable Explorer
shows **Workspace → Explorations → Stages**, with trails inside each Exploration
and reusable pipeline definitions at Workspace level.

1. Choose **Create workspace**, enter a name, then create an **Exploration**.
   Its first local Stage opens as a notebook. **New Stage** adds another step
   without importing a Challenge.
2. Choose **Add image**, enter `Gur xrl vf va gur cvkryf.` as its caption and
   select a PNG file up to 128 KiB. The image and caption share one image Block;
   the caption remains an independently referencable text Asset. **Add text**
   creates a separate text Block.
3. Keep **ROT13** selected and choose **Run cell** below the caption. On macOS the
   real Core operation produces `The key is in the pixels.` beside the cell,
   with its source Asset revision and session attempts.
4. Click the image or caption to open its own **Trail canvas** tab. The caption
   canvas shows the bound Asset, reusable Pipeline definition and session result.
   The image canvas supports inspection and zoom.
5. Write reasoning, then choose **Save reasoning** or press Cmd/Ctrl+S. Create a
   second Exploration and add its own content and notes. Quit and reopen the app:
   saved content, reasoning and the last opened Stage remain separate. Unsaved
   reasoning requires a choice before closing or quitting.

| Action | macOS | Windows/Linux |
| --- | --- | --- |
| Toggle Explorer | Cmd+B | Ctrl+B |
| Open command palette | Cmd+Shift+P | Ctrl+Shift+P |
| Run the active text cell | Cmd+Enter | Ctrl+Enter |
| Save reasoning | Cmd+S | Ctrl+S |

Workspace, Exploration, Stage, Block, Asset, Note, Trail and initial Pipeline
records live in the engine's private SQLite store. There is no seeded example
workspace. See [native storage](reference/desktop-storage.md) for its generation,
immutable revisions, ownership, failure behavior and development ceilings.
Operation selection drafts, open canvas tabs and operation responses remain in
memory; responses are not retained PipelineRuns or official Challenge evaluation.
The renderer is a presentation adapter under the
[desktop domain contract](reference/desktop-domain.md). Each cell or canvas shows
its own result.

## Developer quick start

```sh
npm ci --prefix frontend
npm run build --prefix frontend
cargo run --locked -p play-local
```

Open the private session link printed by the server. Stop it with Ctrl+C. Native development does not start the container terminal.

## Development

For frontend hot reload, run these commands in two terminals from the root:

```sh
CUBE_PLAY_ORIGIN=http://127.0.0.1:5173 cargo run --locked -p play-local
npm run dev --prefix frontend
```

In PowerShell, set `$env:CUBE_PLAY_ORIGIN = 'http://127.0.0.1:5173'` before the Cargo
command. Open the printed session URL after Vite is ready. Vite proxies local API
requests to port 7331 and preserves the browser host. Both listeners use loopback.
To test production serving, build the frontend and start the backend without the
development origin override.
To exercise files, use the launcher or configure both `CUBE_PLAY_SOURCE` and
`CUBE_PLAY_WORKSPACE` (a regular JSON file with a v4 UUID `id` and display `name`).
Set `CUBE_PLAY_DATA` to enable the persisted file workspace and imports; SQLite uses its sibling `state/`
directory, or an explicit `CUBE_PLAY_STATE` override. See [local persistence](persistence.md)
for private directory permissions, migration rules and recovery. The browser harness manages its own
temporary fixtures automatically. See the [typed Lab API](lab-api.md) for
discovery, requests and resource limits; its integration tests start the real
server and real Core worker, including cancellation, binary results and restart recovery.

## Containers

```sh
docker compose up --build --wait
docker compose logs play-local
docker compose down
```

Open the private link in the logs. `CUBE_PLAY_PORT` changes the host port. Container testing requires the isolation prerequisites in [Installation](installation.md). See [Contributing](../CONTRIBUTING.md) for checks.
