# Local launcher

## Open and resume

Run `cube play [path]` after [source installation](../README.md#start-an-investigation).
The optional path defaults to the calling directory. An empty directory opens normally.
A regular file opens its containing directory and passes a file selection to the UI.
`.cube` files carry a challenge intent; this launcher does not parse or execute their
contents. Stage import and validation belong to the challenge capability.

```sh
cube play
cube play ./investigation
cube play 'sample video.mp4'
cube play rabbit.cube
cube play --no-browser ./investigation
cube play --offline ./investigation
cube play --stop ./investigation
```

Arguments are passed literally after your shell parses them. No ordinary startup or
shutdown requires Compose commands. Startup selects a loopback port, waits for Docker
health, retrieves the current per-process session and opens the browser. Failed browser
opening prints guidance and retains the ready session link. Do not share that link.

Opening an active workspace reuses its healthy container without restarting it. A local
registry gives each canonical source directory a random stable UUID; aliases of the same
directory reuse it. Paths stay in the private host registry, while Docker labels use UUIDs.
Separate named data/artifact volumes survive `--stop` and container recreation. No normal
launcher operation deletes volumes. Moving a source creates a different path identity;
the old volumes are retained for explicit recovery. If the original directory disappears,
stop it by its original absolute directory path.

Workspaces run independently. Concurrent starts for one workspace are locked; a startup
already in progress returns bounded retry guidance. Port collisions receive bounded
retries. Corrupt registry data, unexpected owners, changed images and unsafe container
configuration fail clearly; the launcher never silently deletes state or broadens mounts.
After an image update, stop that workspace explicitly before restarting it.

## Install and work offline

The default image is local to the launcher version, runtime revision and UID/GID. First use builds
it from the source location recorded when the launcher was compiled. To prepare it
explicitly or use another trusted checkout:

```sh
cube-play --install-runtime --runtime-source /absolute/path/to/play
cube play --offline ./investigation
```

An installed runtime works without Cloud services or application image downloads.
`--offline` never starts a build or pull. Setup downloads are separate from investigations.
`CUBE_PLAY_IMAGE` can select an explicitly trusted preinstalled image; its protocol,
version and non-root user must match. Images are pinned by immutable ID for each start.
Runtime revision 6 includes interactive visual inspection, transient Core probes, the bounded typed Lab API, managed result bundles and
Core worker alongside the PTY runner and persistent workspace. An
older cached foundation image cannot silently satisfy that contract. The Cube invocation protocol remains 1.
An already-running older image returns explicit stop/restart guidance; its workspace
identity and volumes are preserved.

Source builds consume a trusted sibling Cube 0.18.0 checkout. Set
`CUBE_PLAY_CORE_SOURCE` to another absolute checkout for Docker builds if needed;
native Cargo development uses the documented sibling layout. The build context
includes only workspace metadata and the required Rust build sources, with no deploy key or Docker
socket passed into containers. A missing engine checkout fails with setup guidance.

`CUBE_PLAY_HOME` optionally selects an absolute private launcher-state directory.
The default uses the platform's per-user local application-data location. Unix state
directories require owner-only access; existing shared directories and symlinked state
files are rejected. Registry writes are atomic. This small host configuration is
separate from the SQLite investigation persistence capability.

## Platforms and permissions

| Environment | Behavior |
| --- | --- |
| Linux local Engine | Uses the calling user's UID and primary GID in a non-root image; source ownership and modes remain unchanged. Use a normal account with Docker access. |
| macOS Docker Desktop | Uses Linux containers and host UID/GID with Docker Desktop file sharing. Exact `/host_mnt` VM translations are recognized during recreation. Paths with spaces, Unicode and dollar signs remain literal. |
| Windows Docker Desktop | Uses Linux containers through a local named-pipe context, an unprivileged runtime UID and explicit drive-path conversion. Use a local shared drive; UNC mounts are rejected. |
| WSL | Run Cube, the launcher and Docker CLI inside the same WSL environment with Docker Desktop integration enabled. Windows/native and WSL installations have separate local registries. |

Docker TCP/SSH contexts are rejected, including loopback TCP endpoints; use a local
Unix socket or Docker Desktop named pipe. A filesystem root, special file or directory
containing the active Docker control socket is not an authorized source. Application
containers receive only the selected read-only source, one read-only context file and
their named volumes, with no Docker socket or host process access.

Private `0700` directories and `0600` files work when the runtime's UID/GID maps to their
host owner. Supplemental host groups are not granted. Rootless/user-namespace-remapped
engines and unusual sharing configurations can deny source access; startup fails rather
than becoming root or changing source permissions. Choose a compatible sharing/ownership
configuration. A runtime is not an absolute sandbox against a hostile host account.

macOS uses the system browser opener, Linux uses a trusted absolute `xdg-open` lookup,
and Windows uses its browser association. Explicit `CUBE_PLAY_BROWSER` and
`CUBE_PLAY_DOCKER` overrides require trusted absolute executable paths. Default host
lookups skip relative PATH entries and executables inside the investigation.

## Cube integration contract

Cube finds `cube-play` beside its own executable, or through the trusted absolute
`CUBE_PLAY_LAUNCHER` override. It first calls `--launcher-protocol`, which emits exactly
`cube-play-launcher 1` followed by a newline, without accessing Docker or state.
The invocation is `cube-play --protocol 1 [flags] [-- path]`; the working directory and
OS path arguments are preserved. Missing/incompatible launchers give installation
guidance. `--json-errors` uses the schema-versioned Cube error envelope.

Unix delegation replaces the Cube process. On Windows the launcher shares the console
and Cube waits for its conventional exit status. Ctrl+C/termination interrupts startup,
kills owned host-command groups (Unix) or jobs (Windows), and stops a newly started
owned runtime when necessary. Canceling reuse does not stop an existing investigation.
An explicit `--stop` affects only the registered workspace and preserves its volumes.

The launcher starts/stops both owned containers and checks Runner privileges,
network isolation, private mounts and health before reusing them. Runtime setup
requires a Landlock-capable engine; see [terminal requirements](terminal.md).
The capability probe uses no host mounts and returns actionable upgrade guidance.
Full Lab/challenge journeys remain part of
[integrated MVP validation](https://github.com/cubearc/play/issues/11).
