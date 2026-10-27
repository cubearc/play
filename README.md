# Cube Play

A local-first investigation workspace powered by the [Cube Rust engine](https://github.com/cubearc/cube).
Free Lab is the general workspace; structured challenges are optional.

The host launcher opens local directories or files in a protected browser application.
Browse files, preview supported media/text, inspect original bytes and explicitly
import private copies without changing the source. Investigation names and file
navigation persist in a private local SQLite database. See [local persistence](docs/persistence.md)
for storage limits, migrations and recovery. The [isolated Cube terminal](docs/terminal.md)
executes real CLI commands. The [typed Lab API](docs/lab-api.md) exposes Core operations
and managed results. The [visual Lab](docs/visual-lab.md) provides exact pixel,
histogram, channel and transformation controls. The [notebook](docs/notebook.md)
autosaves private notes and reopens named artifacts/history. Optional
[local challenges](docs/challenges.md) provide inert instructions, stage navigation,
private progress and explicit Lab-result candidate associations. Planning stays in
[GitHub Issues](https://github.com/cubearc/play/issues) and
[CubeArc Roadmap](https://github.com/orgs/cubearc/projects/1).

## Start an investigation

Install a Landlock-capable Docker Engine with Compose v2, or Docker Desktop 4.94.0+
with Linux containers, and start the local engine. For source installation, use
Rust stable through rustup and Node.js 24 with npm. Build the trusted launcher UI
before compiling the launcher. With Cube
and Play checked out side by side, run from this repository:

```sh
npm ci --prefix frontend
npm run build --prefix frontend
cargo install --locked --path ../cube/crates/cube-cli
cargo install --locked --path crates/play-launcher
cube play
```

Native builds depend directly on the sibling Cube workspace's `cube-core = 0.18.0`;
the library is not yet published on crates.io. Keep that trusted checkout available
for builds. Docker receives the required Cube build sources through a named
context; algorithms stay in Core and deploy keys stay on the host.

The first launch builds a local runtime image from the trusted Play checkout;
setup can download pinned base images and locked dependencies. Keep the checkout
available until setup finishes, or specify its absolute path with
`CUBE_PLAY_RUNTIME_SOURCE`. No application image is assumed to be published.

```sh
cube play ./investigation
cube play sample.mp4
cube play rabbit.cube
cube play --stop ./investigation
```

The browser opens after readiness. Empty directories work, source mounts are read-only,
and separate local volumes survive shutdown/recreation. Direct files select their
parent workspace and open an inspector. Challenge stages remain a separate
capability; selecting `.cube` never executes it. Read the
[workspace guide](docs/workspace.md) for previews, explicit imports and limits.
Use `--no-browser` to print a private session link or `--offline` to require an
already installed runtime without setup downloads. See the [launcher guide](docs/launcher.md)
for platform support, recovery and the invocation contract.

Use **Workspaces** in the application header to create an empty directory, open an
existing one through the system folder dialog, or resume a recent investigation.
These actions run in the trusted host launcher; containers receive no host-management
authority. A source installation needs `zenity` on Linux for the folder dialog.

## Developer quick start

Install [Rust stable through rustup](https://rustup.rs/) and Node.js 24 with npm.
From the repository root:

```sh
npm ci --prefix frontend
npm run build --prefix frontend
cargo run --locked -p play-local
```

Open the session URL printed by the server. It uses `127.0.0.1:7331` and a temporary
credential in the URL fragment. The UI removes it before requests and keeps the
validated capability in origin-scoped, per-tab session storage. API requests use
Authorization headers, so sessions at different local ports do not share cookies.
Refresh reuses that tab's session; restart requires the new link. Use the launcher
link when opening a separate tab.
Do not share session links. Stop the application with Ctrl+C.

Readiness is available at `http://127.0.0.1:7331/healthz`; `/api/health` requires the
local session. A plain browser visit without the session link shows recovery guidance.

Open an image with **Open visual Lab** for Core-backed pixel values, histograms,
channel previews and exact transformations. See the [visual Lab guide](docs/visual-lab.md).

Use [typed explorations](docs/explorations.md) to save hypotheses, connect operation
ports in Flow or Steps view and preserve execution history across restarts.

Import a selected `.cube` JSON manifest or run `cube play rabbit.cube` for optional
[local challenge stages](docs/challenges.md). Workspaces remain usable without a challenge.

## Docker development

Docker Engine/Desktop and Docker Compose v2 are optional for native development and
required to test the container runtime. These commands are for contributors:

```sh
docker compose up --build --wait
docker compose logs play-local
docker compose down
```

Open the session link in the logs. Set `CUBE_PLAY_PORT` to change the host port.
Compose publishes only on loopback and runs the application non-root with a read-only
filesystem and bounded resources. No user directories or Docker socket are mounted.
The runtime starts Play Local and a network-disabled Cube Runner with private
Unix transport. A Landlock-capable engine is required; use Docker Desktop 4.94.0 or
newer. The host launcher manages ordinary-user startup and shutdown.

See [CONTRIBUTING](CONTRIBUTING.md) for development and quality commands,
[Architecture](docs/architecture.md) for boundaries, and [SECURITY](SECURITY.md)
for vulnerability reporting. Licensed under [Apache-2.0](LICENSE).
