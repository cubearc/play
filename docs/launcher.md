# Create and reopen Workspaces

<a id="choose-a-workspace-in-the-browser"></a>
<a id="cube-integration-contract"></a>
<a id="install-and-work-offline"></a>
<a id="open-and-resume"></a>
<a id="open-and-resume-workspaces"></a>

A Workspace groups independent Explorations in private local app storage. In the
Electron welcome screen, enter a name and choose **Create workspace**, then
**New Exploration** to begin a notebook. Creating one does not authorize an
entire source directory or import its files.

Use **Workspaces** in the Explorer to return to the workspace chooser and open a
saved entry or create another. Save reasoning before switching. The development
app has one active Workspace shared by up to eight workbench windows. Use
**File → New Window** for an independent Exploration/Stage selection. Close the
other windows before changing Workspaces. Runs remain with the engine when a
window closes, including the last window. Open the app or source folder again to
restore a window. Use **Quit Cube Play** on macOS or **File → Quit** on Linux
to end the engine and revoke external connections. [External terminals](terminal.md#use-an-external-terminal) attach
through an explicit temporary context file.

Reopening the app restores the last successfully opened Workspace, Exploration
and selected Stage. Workspace identity comes from the engine, rather than a
source path. [Saved state](persistence.md) explains recovery and what remains
only in memory.

## Platforms and permissions

The Electron preview enables operation execution on macOS and Linux x64/ARM64.
Linux requires Landlock and seccomp; missing enforcement refuses execution.
Windows workers and PTYs remain incomplete. See
[installation](installation.md) for the actual source workflow; the Electron path
requires neither a browser session URL nor Docker/WSL setup.

Choose individual PNGs through **Add image** in a Stage. The trusted native dialog
passes only the selected file for bounded import; it does not grant the renderer
a filesystem path API. See [Assets and imports](workspace.md).

For exact app-data layout, locks and refusal rules, use
[native storage](reference/desktop-storage.md). Signed installation, update and
uninstall behavior remain separate desktop deliveries.

## Open a source folder or file

Use **File → Open Folder…**, or open a folder through the trusted Cube CLI's
`cube play /absolute/source/folder` command. Play associates that canonical folder
with a private managed Workspace. Reopening the folder selects the same store;
it does not import the directory, use an adjacent database or modify source files.
The CLI requires an installed sibling `cube-play` executable or an explicit
trusted `CUBE_PLAY_APP` path.

A native file-open intent imports a bounded PNG or file into the selected Stage.
A canonical Challenge JSON package creates an independent Exploration. Invalid
packages are refused before creating a Workspace. Native intents never execute
an operation automatically. **File → Copy Workspace Link** copies a local link
to the current Workspace/Exploration/Stage; opening it asks for confirmation and
only selects existing local identities. It is not a shareable Challenge download.
