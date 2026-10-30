# Native terminal boundary

<a id="private-transport-and-browser-access"></a>
<a id="session-and-resource-boundaries"></a>
<a id="terminal-resource-and-transport-boundaries"></a>

The Electron preview supports attached contextual CLI commands through private
pipes. It has no integrated PTY or external terminal attachment. The native
contract distinguishes the implemented channel from the remaining terminal boundary.

- [Native process and CLI contract](desktop-runtime.md)
- [Current terminal availability](../terminal.md)
