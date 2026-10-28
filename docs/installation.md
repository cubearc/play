# Install Cube Play from source

Use stable Rust through rustup, Node.js 24.15+ with npm and a running local Docker
engine with Compose v2. The Linux engine must support Landlock ABI 3; for Docker
Desktop use 4.94.0 or newer with Linux containers.

On Windows with Docker Engine installed inside WSL, run the installation and
`cube play` commands inside that distribution. Select Windows files through their
WSL paths, such as `/mnt/d/investigation`. Docker Desktop is optional for this
workflow; the Engine must provide the same Linux isolation requirements.

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

Linux folder selection also needs `zenity`. In WSL, use a desktop dialog
environment or pass an explicit source path. See [platform permissions](launcher.md#platforms-and-permissions)
for the native Windows and WSL connection boundaries, and [verification](reference/validation.md)
for the recorded scope of local checks.

Continue with [your first inspection](getting-started.md).
See [launcher options](launcher.md) for explicit runtime installation, offline use
and platform permissions, or [Development](development.md) for native contributors.
