# Cube Play documentation

Play's Electron source preview is a local investigation IDE. Start with
[installation](installation.md) and [your first notebook](getting-started.md).
The Electron preview enables operations on macOS. Linux workers have a separate
[native validation workflow](development.md#validate-linux-workers); Linux
Electron/terminal integration and Windows worker enforcement remain incomplete.
Signed release packages are not available through this source workflow.

| Goal | Read |
| --- | --- |
| Create, switch and reopen local work | [Workspaces](launcher.md), [saved state](persistence.md) |
| Organize independent investigations and canvases | [Explorations and Trails](explorations.md) |
| Add text or an image with its caption | [Assets and imports](workspace.md) |
| Run a text cell and save reasoning | [Stage notebook](notebook.md) |
| Inspect an image in its own canvas | [Image inspection](visual-lab.md) |
| Explore the terminal and local/official evidence | [Challenges](challenges.md), [Candidates and Solutions](solutions.md), [terminal](terminal.md) |
| Develop and verify the native app | [Development](development.md), [Contributing](../CONTRIBUTING.md) |

The executor discovers eligible operations from the Core registry and supports typed
graphs with named Asset bindings and two concurrent independent branches per Run.
The [Cube terminal](terminal.md) uses the same scoped Run coordinator.
Two Runs can execute across Explorations with bounded queue admission. Complete
native release delivery remains in the
[local milestone](https://github.com/cubearc/play/milestone/3); Cloud participation
belongs to the [independent Cloud project](https://github.com/cubearc/play-platform/issues/10).
Catalog discovery alone does not make an operation executable in this preview.

Exact ownership, IPC, storage and Run contracts are grouped under Reference in
[the navigation](SUMMARY.md). Algorithms and operation metadata belong to Cube.
