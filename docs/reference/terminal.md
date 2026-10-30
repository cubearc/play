# Native terminal boundary

<a id="private-transport-and-browser-access"></a>
<a id="session-and-resource-boundaries"></a>
<a id="terminal-resource-and-transport-boundaries"></a>

The Electron preview supports attached contextual CLI commands through private
pipes and an interactive macOS PTY with a separate private control socket.
External terminal discovery and Linux/Windows PTY enforcement remain separate
work. The runtime contract records implemented limits and remaining boundaries.

- [Native process and CLI contract](desktop-runtime.md)
- [Use the Cube terminal](../terminal.md)
