# Launcher and runtime contracts

## Workspace identity and lifetime

Opening an active workspace reuses its healthy container without restarting it. A local
registry gives each canonical source directory a random stable UUID; aliases of the same
directory reuse it. Paths stay in the private host registry, while Docker labels use UUIDs.
Separate named data/artifact volumes survive `--stop` and container recreation. No normal
launcher operation deletes volumes. Moving a source creates a different path identity;
the old volumes are retained for explicit recovery. If the original directory disappears,
stop it by its original absolute directory path.

## Host chooser boundary

The chooser is a separate loopback origin served by the native launcher, with its
own per-tab capability. Its React assets are built into the launcher from trusted
sources. Neither this credential nor host paths are passed to the application
containers. The application receives only a non-secret return origin. Requests
are closed versioned intents (`open`, `new` with a portable folder name, or `recent`
with a registered UUID), never arbitrary browser-supplied host paths or commands.
Exact Host/Origin validation, omitted cookies, bounded bodies, a five-second body
deadline and one active action protect the boundary. The native folder picker and
Docker commands retain bounded output, deadlines and process-group cleanup.

`--no-browser` prints the direct application URL without starting a chooser.
Development sessions and these direct links have no Workspaces button unless
they previously received a chooser handoff. To stop the idle host chooser without
stopping containers or removing data, run `cube-play --stop-chooser`. A later normal
launch restarts it. Preserve private launcher state if its control record or
registry is invalid. These boundaries do not isolate a compromised host account.

Workspaces run independently. Concurrent starts for one workspace are locked; a startup
already in progress returns bounded retry guidance. Port collisions receive bounded
retries. Corrupt registry data, unexpected owners, changed images and unsafe container
configuration fail clearly; the launcher never silently deletes state or broadens mounts.
After an image update, stop that workspace explicitly before restarting it.

On Windows, background helpers use an explicit handle list and inherit no caller handles or output pipes. The chooser remains independent of the invoking CLI.

## Runtime source and private state

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

Runtime revision 22 includes private raw AES blocks and stream transforms,
typed Enigma I/M3 and classical-analysis operations.
It retains the verified container PID budget for Runner sessions, so unrelated
host/browser processes sharing its UID do not prevent terminal startup. Typed
operations, explicit terminal captures, retained results and field connections
are described in the [Lab API](../lab-api.md) and [explorations](../explorations.md).
Older runtime images cannot satisfy that contract; the launcher selects a
revision-specific image and rejects incompatible image labels.

## Platform boundaries

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
requires a Landlock-capable engine; see [terminal requirements](../terminal.md).
The capability probe uses no host mounts and returns actionable upgrade guidance.
Full Lab/challenge journeys remain part of
[integrated MVP validation](https://github.com/cubearc/play/issues/11).
