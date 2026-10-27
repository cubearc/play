# Install Cube Play from source

Use stable Rust through rustup, Node.js 24.15+ with npm and a running local Docker
engine with Compose v2. The Linux engine must support Landlock ABI 3; for Docker
Desktop use 4.94.0 or newer with Linux containers.

Keep trusted Cube and Play checkouts side by side. Play consumes
`cube-core = 0.18.0` directly from `../cube`; no published crate or application
image is assumed. From the Play repository:

```sh
npm ci --prefix frontend
npm run build --prefix frontend
cargo install --locked --path ../cube/crates/cube-cli
cargo install --locked --path crates/play-launcher
cube play
```

Build the trusted launcher assets before compiling it. The first launch builds a
local runtime image and can download pinned base images and dependencies.
Keep the Play checkout available until setup completes, or set
`CUBE_PLAY_RUNTIME_SOURCE` to its absolute path.

Linux folder selection also needs `zenity`. Native Windows and WSL/Docker
Desktop validation remains deferred in [the platform follow-up](https://github.com/cubearc/play/issues/24);
the platform instructions describe intended behavior.
Recorded validation covers native macOS and Linux-container workflows.

Continue with [your first inspection](getting-started.md).
See [launcher options](launcher.md) for explicit runtime installation, offline use
and platform permissions, or [Development](development.md) for native contributors.
