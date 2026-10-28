# Isolated Cube terminal

Open a workspace with `cube play`, select **Terminal**, choose a working directory and select **Start
terminal**. The console executes the real versioned `cube` binary in Cube Runner.
Try `cube encode base64 hello` or `cube --help`. Quote arguments containing spaces.
Use Ctrl+C to interrupt a command; `exit`, Ctrl+D at an empty prompt or **End session**
closes the session and its processes. Terminal input/output stays in memory and is
not automatically recorded, uploaded or added to investigation history.
Switching workspace sections keeps an active session running. Return to Terminal
to inspect its output or explicitly end it.

The console accepts `cube ...`, `pwd`, `clear`, `exit` and
`cd <source|imports|output> [relative/path]`. It does not evaluate shell scripts,
expansions, pipelines or redirects. Use Cube's explicit output options to create
files in the output directory. The host-only `cube play` command is unavailable
inside Runner. PTY bytes are forwarded without UTF-8 conversion; terminal line
discipline still applies. The Lab never parses terminal presentation into typed
operation results.

## Working directories

| Store | Runner path | Access |
| --- | --- | --- |
| Source | `/workspace/source` | Authorized selected host directory, mounted read-only |
| Imports | `/workspace/data/imports` | Explicitly imported files, read-only |
| Output | `/workspace/output` | Separate persistent artifact volume, writable |

Directory selection accepts bounded relative paths and rejects symlinks, traversal,
Windows device names and missing directories. CLI file access is restricted by
Linux Landlock: it can read sources/imports, write output and its private temporary
directory, and load the trusted CLI/system libraries. A symlink cannot grant access
to a private target outside those roots. The data volume is mounted read-only to
Runner; **its SQLite database, notes and state are outside the CLI's Landlock
allowlist**. Only the trusted coordinator sees that parent mount. CLI processes
cannot open the database even through a source symlink.

A compatible engine must provide **Landlock ABI 3 with Landlock enabled**. Docker
Desktop enables that LSM in [4.94.0](https://docs.docker.com/desktop/release-notes/#4940)
and newer releases. Linux engine users need a compatible kernel/LSM configuration.
The launcher probes the actual runtime before startup; the runner checks again
before accepting sessions. Unsupported isolation fails explicitly. There is no
unrestricted fallback, privileged mode or container Docker socket.

## Session and resource boundaries

Sessions have explicit time, output and process limits. An unsupported isolation environment fails before terminal startup. See [exact resource bounds](reference/terminal.md#session-and-resource-boundaries).

## Private transport and browser access

Open the application with its private session link. See [transport and authorization](reference/terminal.md#private-transport-and-browser-access) for the local security contract.

## Send a computed result to the Lab

Request capture explicitly on a supported command:

```sh
cube --lab-capture text reverse --text CUBE
cube --lab-capture decode hex --text 00ff80 --output raw
cube --lab-capture image histogram sample.png --max-colors 100 --json
```

`image invert` also supports capture with its ordinary explicit output option.
Other commands remain available without capture. **Refresh staged captures** lists
only this terminal session's private staging directory. **Send to Lab** validates
the selected capture and publishes its exact named outputs without rerunning the
command. Inspect the typed result and save a named artifact to reuse it in an
exploration. Repeated publication of identical captured bytes returns the same
immutable artifact. Publication failures preserve existing artifacts and staging.

Staging is limited to 16 capture files, each at most 128 KiB; the existing private
session tmpfs also bounds storage. Ending/disconnecting the terminal removes
unpublished staging and its processes. Published artifacts persist separately and
survive restart/recreation. No capture is made unless requested, and refreshing
never publishes it. PTY bytes and ANSI presentation are never parsed as results.

The private Runner transport sends bounded capture chunks only from the current
session directory; IDs cannot authorize other sessions or arbitrary paths. Play
checks format/operation/schema/Core versions, named outputs and exact digests with
the canonical Core capture validator. Sources, private database paths and terminal
history are not imported. A CLI capture is an **unverified declaration**, even when
its structure and digests pass. Missing original input values and incomplete command
history remain explicit replay gaps; publication does not infer replay sources.

Outside Runner, use `--lab-capture=PATH`, then explicitly select that file in Files
and use **Import selected CLI capture** in Explorations. Only already authorized
source/import files are read, with the same bounds and unverified provenance policy.
