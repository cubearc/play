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
app has one active Workspace and one trusted window; it does not attach multiple
windows or external CLI sessions to a shared engine.

Reopening the app restores the last successfully opened Workspace, Exploration
and selected Stage. Workspace identity comes from the engine, rather than a
source path. [Saved state](persistence.md) explains recovery and what remains
only in memory.

## Platforms and permissions

The source preview's operation execution is available on macOS. Windows/Linux
workers reject execution until native enforcement is validated. See
[installation](installation.md) for the actual source workflow; the Electron path
requires neither a browser session URL nor Docker/WSL setup.

Choose individual PNGs through **Add image** in a Stage. The trusted native dialog
passes only the selected file for bounded import; it does not grant the renderer
a filesystem path API. See [Assets and imports](workspace.md).

For exact app-data layout, locks and refusal rules, use
[native storage](reference/desktop-storage.md). Signed installation, update and
uninstall behavior remain separate desktop deliveries.
