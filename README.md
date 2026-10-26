# Cube Play

A local-first investigation workspace powered by the [Cube Rust engine](https://github.com/cubearc/cube).
Free Lab is the general workspace; structured challenges are optional.

The host launcher opens local directories or files in a protected browser application.
Browse files, preview supported media/text, inspect original bytes and explicitly
import private copies without changing the source. The Cube terminal, visual Lab,
investigation persistence and structured challenges
are tracked in [GitHub Issues](https://github.com/cubearc/play/issues) and
[CubeArc Roadmap](https://github.com/orgs/cubearc/projects/1).

## Start an investigation

Install Docker Engine/Desktop with Linux containers and Compose v2, and start the
local engine. For source installation, use Rust stable through rustup. With Cube
and Play checked out side by side, run from this repository:

```sh
cargo install --locked --path ../cube/crates/cube-cli
cargo install --locked --path crates/play-launcher
cube play
```

Native builds depend directly on the sibling Cube workspace's `cube-core = 0.18.0`;
the library is not yet published on crates.io. Keep that trusted checkout available
for builds. Docker receives only its workspace manifest and library sources through
a named context; algorithms stay in Core and deploy keys stay on the host.

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
parent workspace and open an inspector. Challenge stages and the terminal are
delivered separately; selecting `.cube` never executes it. Read the
[workspace guide](docs/workspace.md) for previews, explicit imports and limits.
Use `--no-browser` to print a private session link or `--offline` to require an
already installed runtime without setup downloads. See the [launcher guide](docs/launcher.md)
for platform support, recovery and the invocation contract.

## Developer quick start

Install [Rust stable through rustup](https://rustup.rs/) and Node.js 24 with npm.
From the repository root:

```sh
npm ci --prefix frontend
npm run build --prefix frontend
cargo run --locked -p play-local
```

Open the session URL printed by the server. It uses `127.0.0.1:7331` and a temporary
credential in the URL fragment. The UI removes the fragment and exchanges it for an
HttpOnly, same-site cookie. Refresh reuses the cookie; restart requires the new link.
Do not share session links. Stop the application with Ctrl+C.

Readiness is available at `http://127.0.0.1:7331/healthz`; `/api/health` requires the
local session. A plain browser visit without the session link shows recovery guidance.

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
The current runtime starts Play Local; Cube Runner arrives with the isolated
terminal capability. The host launcher manages ordinary-user startup and shutdown.

See [CONTRIBUTING](CONTRIBUTING.md) for development and quality commands,
[Architecture](docs/architecture.md) for boundaries, and [SECURITY](SECURITY.md)
for vulnerability reporting. Licensed under [Apache-2.0](LICENSE).
