# Open and resume workspaces

## Open and resume

Run `cube play [path]` after [source installation](../README.md#start-an-investigation).
The optional path defaults to the calling directory. An empty directory opens normally.
A regular file opens its containing directory and passes a file selection to the UI.
`.cube` files carry a challenge intent; this launcher does not parse or execute their
contents. The application validates and imports the [non-executable local manifest](challenges.md)
after its authorized session opens; the launcher never runs it.

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

## Choose a workspace in the browser

The launcher opens a recent-workspace list with **New workspace** and **Open folder**
actions. **New workspace** reveals the folder-name form; **Create workspace** then
opens the system dialog to choose its parent directory. **Cancel** closes the form
without opening a dialog and retains the draft name. **Open folder** immediately
opens the directory dialog. Recent entries resume their existing local investigation.

After a normal `cube play` launch, **Workspaces** in the application header opens
the trusted host chooser. **Create workspace** creates exactly one new directory
under the parent selected in the system folder dialog. Existing directories are
never overwritten. **Open folder** authorizes an existing folder,
including an empty one. Recent entries reuse the registered identity and volumes;
unavailable sources remain listed with explicit recovery guidance. A recent root
that now resolves through a different symlink must be authorized again with Open.

The chooser starts or reuses the same two-container runtime and redirects only
after application readiness. Cancelling a dialog preserves all investigation data.
If startup fails after creating a folder, the folder is retained; choose Open to
retry it. Linux desktop selection requires `zenity`; macOS uses the system folder
dialog and Windows uses its native folder browser. WSL requires a desktop dialog
environment or an explicit path through `cube play [path]`.

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
The runtime image must match the installed launcher's version and protocol. An already-running older image returns explicit stop/restart guidance; workspace identity and volumes are preserved. See [runtime contracts](reference/launcher.md).

## Platforms and permissions

Recorded validation covers native macOS checks and real Linux-container
workflows on Docker Desktop. The maintainer deferred native Windows and WSL/Docker
Desktop testing on 2026-10-27. The Windows and WSL rows below describe intended
behavior; validation remains open in [the platform follow-up](https://github.com/cubearc/play/issues/24).

| Environment | Behavior |
| --- | --- |
| Linux local Engine | Uses the calling user's UID and primary GID in a non-root image; source ownership and modes remain unchanged. Use a normal account with Docker access. |
| macOS Docker Desktop | Uses Linux containers and host UID/GID with Docker Desktop file sharing. Exact `/host_mnt` VM translations are recognized during recreation. Paths with spaces, Unicode and dollar signs remain literal. |
| Windows Docker Desktop | Uses Linux containers through a local named-pipe context, an unprivileged runtime UID and explicit drive-path conversion. Use a local shared drive; UNC mounts are rejected. |
| WSL | Run Cube, the launcher and Docker CLI inside the same WSL environment with Docker Desktop integration enabled. Windows/native and WSL installations have separate local registries. |

Use a local Docker context and an ordinary account with access to it. Source access can fail under unsupported sharing or ownership settings; see [platform and runtime boundaries](reference/launcher.md#platform-boundaries).

## Cube integration contract

The CLI delegates to the separately installed launcher. See the [versioned invocation and shutdown contract](reference/launcher.md#cube-integration-contract).
