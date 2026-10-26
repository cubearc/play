# Cube Play

A local-first investigation workspace powered by the [Cube Rust engine](https://github.com/cubearc/cube).
Free Lab is the general workspace; structured challenges are optional.

The application foundation serves a React interface and a protected local Rust API.
Workspace exploration, the Cube terminal, visual analysis, persistence and challenges
are tracked in [GitHub Issues](https://github.com/cubearc/play/issues) and
[CubeArc Roadmap](https://github.com/orgs/cubearc/projects/1).
The `cube play [path]` launcher is tracked in [#2](https://github.com/cubearc/play/issues/2).

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
The current foundation starts Play Local; Cube Runner arrives with the isolated
terminal capability. Ordinary-user Docker lifecycle belongs to the upcoming launcher.

See [CONTRIBUTING](CONTRIBUTING.md) for development and quality commands,
[Architecture](docs/architecture.md) for boundaries, and [SECURITY](SECURITY.md)
for vulnerability reporting. Licensed under [Apache-2.0](LICENSE).
