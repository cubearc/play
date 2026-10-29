# Cube terminal availability

<a id="isolated-cube-terminal"></a>
<a id="private-transport-and-browser-access"></a>
<a id="send-a-computed-result-to-the-lab"></a>
<a id="session-and-resource-boundaries"></a>
<a id="working-directories"></a>

The Electron preview has no integrated native terminal, PTY or contextual CLI
attachment. Its text cells invoke real Core operations through the Rust engine;
they are not shell commands. Use [the notebook](notebook.md) for current execution.

The target native terminal will run the real Cube CLI with explicit Workspace,
Exploration, Asset and Pipeline context, under the same owned execution boundary.
It must not give the renderer a shell, database path or unrestricted filesystem
API. The [IPC and native execution contract](reference/desktop-runtime.md) owns
that design. Track implementation in [#48](https://github.com/cubearc/play/issues/48).

A Cube CLI installed separately can be used according to
[Cube's CLI guide](https://github.com/cubearc/cube/blob/main/docs/interfaces/cli.md),
but its output does not automatically enter this Electron notebook or create a
PipelineRun. Do not use a browser/container terminal setup as an Electron fallback.
