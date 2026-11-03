import { randomBytes, randomUUID } from "node:crypto";
import { mkdirSync } from "node:fs";
import { readFile, stat } from "node:fs/promises";
import { basename, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  app,
  BrowserWindow,
  clipboard,
  dialog,
  ipcMain,
  Menu,
  protocol,
  session,
} from "electron";
import {
  AssetGrants,
  importChallenge,
  importFile,
  importImage,
} from "./assets.ts";
import { attachmentIntent } from "./attachment-contract.ts";
import { Attachments } from "./attachments.ts";
import { APP_URL, ownsFrame, resourceName } from "./authority.ts";
import { Cli, contextCommand, dispatchCli } from "./cli.ts";
import { cliIntent } from "./cli-contract.ts";
import { contract } from "./contract.ts";
import { Engine } from "./engine.ts";
import { exportIntent } from "./export-contract.ts";
import { NativeExports } from "./exports.ts";
import { readImport } from "./imports.ts";
import { verifyNative } from "./integrity.ts";
import { menuTemplate } from "./menu.ts";
import { nativeFiles } from "./native-files.ts";
import { type OpenIntent, openIntent, workspaceLink } from "./open-intent.ts";
import { SourceRoots } from "./source-roots.ts";
import { terminalIntent } from "./terminal-contract.ts";
import {
  type Context,
  cancelRun,
  challengeImport,
  type ExplorationView,
  fileImport,
  type ImportedChallenge,
  imageImport,
  imageRead,
  request,
  runRequest,
} from "./workspace-contract.ts";

if (process.argv.includes("--desktop-protocol")) {
  await new Promise<void>((resolve) =>
    process.stdout.write("cube-play 1\n", () => resolve()),
  );
  app.exit(0);
}

protocol.registerSchemesAsPrivileged([
  {
    scheme: "cube-play",
    privileges: { standard: true, secure: true, supportFetchAPI: true },
  },
]);
app.enableSandbox();
app.setName("Cube Play");
const profile =
  app.commandLine.getSwitchValue("user-data-dir") ||
  join(app.getPath("appData"), "CubeArc", "Play");
if (!isAbsolute(profile)) throw new Error("invalid_profile");
mkdirSync(profile, { recursive: true, mode: 0o700 });
app.setPath("userData", profile);

const roots = new SourceRoots(profile);
interface OpenJob {
  input: string;
  owner: WindowState | null;
  workspaceId: string | null;
  context: Context | null;
}
const pendingOpens: OpenJob[] = [];
let switchingWorkspace = false;
let opening = false;

const styleNonce = randomBytes(24).toString("base64");
const resources = new URL("../desktop-renderer/", import.meta.url);
const native = new URL("../desktop-native/", import.meta.url);
const files = nativeFiles();
const enginePath = fileURLToPath(new URL(files.engine, native));
const workerPath = files.worker
  ? fileURLToPath(new URL(files.worker, native))
  : undefined;
const attachments = new Attachments();
async function disconnectAttachments() {
  await attachments.clear();
}
let engine: Engine | undefined;
let nativeExports: NativeExports | undefined;
const exportingWindows = new Set<number>();
let cli: Cli | undefined;
interface WindowState {
  window: BrowserWindow;
  grants: AssetGrants;
  dirty: boolean;
  confirming: boolean;
  workspaceId: string | null;
  context: Context | null;
  initialLocation: OpenIntent | null;
  navigationEpoch: number;
  documentEpoch: number;
}
const windows = new Map<number, WindowState>();
let boundWorkspaceId: string | null = null;
const activeWindow = () =>
  windows.get(BrowserWindow.getFocusedWindow()?.webContents.id ?? -1) ??
  windows.values().next().value;
const windowFor = (event: Electron.IpcMainInvokeEvent): WindowState => {
  const state = windows.get(event.sender.id);
  if (!state || !ownsFrame(event, state.window.webContents))
    throw new Error("unauthorized_frame");
  return state;
};
let quitting = false;

if (!app.requestSingleInstanceLock()) app.exit(0);
async function confirmDiscard(state: WindowState): Promise<boolean> {
  if (!state.dirty) return true;
  if (state.window.isDestroyed() || state.confirming) return false;
  state.confirming = true;
  try {
    const result = await dialog.showMessageBox(state.window, {
      type: "warning",
      message: "Changes have not been saved.",
      detail:
        "Keep editing to save your content, or discard the unsaved draft.",
      buttons: ["Keep editing", "Discard draft"],
      defaultId: 0,
      cancelId: 0,
    });
    if (result.response !== 1) return false;
    state.dirty = false;
    state.window.setDocumentEdited(false);
    return true;
  } finally {
    state.confirming = false;
  }
}

function createWindow(
  initialLocation: OpenIntent | null = null,
  internal = false,
): void {
  if (switchingWorkspace && !internal) return;
  if (windows.size >= 8) {
    void dialog.showMessageBox({
      type: "info",
      message: "Eight workbench windows are already open.",
      detail: "Close a window before opening another.",
    });
    return;
  }
  const next = new BrowserWindow({
    width: 1280,
    height: 800,
    minWidth: 800,
    minHeight: 540,
    backgroundColor: "#1f1f1f",
    ...(process.platform === "darwin"
      ? { titleBarStyle: "hiddenInset" as const }
      : {}),
    title: "Cube Play",
    webPreferences: {
      preload: fileURLToPath(new URL("preload.cjs", import.meta.url)),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
      webviewTag: false,
    },
  });
  const state: WindowState = {
    window: next,
    grants: new AssetGrants(),
    dirty: false,
    confirming: false,
    workspaceId: boundWorkspaceId,
    context: null,
    initialLocation,
    navigationEpoch: 0,
    documentEpoch: 0,
  };
  const windowId = next.webContents.id;
  windows.set(windowId, state);
  next.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  next.webContents.on("will-navigate", (event) => event.preventDefault());
  next.webContents.on("will-frame-navigate", (event) => event.preventDefault());
  next.webContents.on("will-attach-webview", (event) => event.preventDefault());
  next.on("close", (event) => {
    if (!state.dirty) return;
    event.preventDefault();
    void confirmDiscard(state).then((discard) => {
      if (discard && !next.isDestroyed()) next.close();
    });
  });
  next.on("closed", () => {
    state.grants.revoke();
    windows.delete(windowId);
  });
  void next.loadURL(APP_URL);
}

function activateWorkbench(internal = false): void {
  if (quitting) return;
  const state = activeWindow();
  if (state) state.window.focus();
  else if (engine) createWindow(null, internal);
}

async function showWorkspace(
  id: string,
  requested: OpenIntent | null = null,
  owner: WindowState | null = activeWindow() ?? null,
): Promise<void> {
  const view = await engine?.workspace({ kind: "view", workspace_id: id });
  const location: OpenIntent = requested ?? {
    workspace_id: id,
    exploration_id: view?.selectedExplorationId ?? null,
    stage_id: view?.selectedStageId ?? null,
  };
  const state = owner;
  if (boundWorkspaceId !== id) await disconnectAttachments();
  boundWorkspaceId = id;
  if (state?.window.isDestroyed()) return;
  if (state) {
    state.grants.revoke();
    state.workspaceId = id;
    state.context = null;
    state.initialLocation = location;
    state.navigationEpoch++;
    state.documentEpoch++;
    state.window.reload();
    state.window.focus();
  } else createWindow(location, true);
}
async function maySwitch(id: string | null): Promise<void> {
  if (boundWorkspaceId && boundWorkspaceId !== id) {
    if (windows.size > 1) throw new Error("workspace_in_use");
    const state = activeWindow();
    if (state && !(await confirmDiscard(state)))
      throw new Error("open_cancelled");
  }
}
async function openWorkspaceLink(
  current: Engine,
  input: string,
  state: WindowState | undefined,
): Promise<void> {
  const intent = openIntent(input);
  await maySwitch(intent.workspace_id);
  const approved = await dialog.showMessageBox({
    type: "question",
    message: "Open this local Workspace?",
    detail:
      "This link selects local content. It cannot run a Pipeline or authorize files.",
    buttons: ["Cancel", "Open Workspace"],
    defaultId: 0,
    cancelId: 0,
  });
  if (approved.response !== 1) return;
  if (boundWorkspaceId !== intent.workspace_id)
    await current.workspace({
      kind: "open",
      workspace_id: intent.workspace_id,
    });
  if (intent.exploration_id)
    await current.workspace({
      kind: "open_exploration",
      workspace_id: intent.workspace_id,
      exploration_id: intent.exploration_id,
      stage_id: intent.stage_id,
    });
  if (windows.size > 1) {
    createWindow(intent, true);
  } else await showWorkspace(intent.workspace_id, intent, state ?? null);
}
async function openSourceFolder(
  current: Engine,
  input: string,
  state: WindowState | undefined,
): Promise<void> {
  const id = await roots.open(input, {
    create: async (key, title) => {
      await maySwitch(null);
      const view = await current.workspace({ kind: "create", key, title });
      return view.id;
    },
    open: async (id) => {
      await maySwitch(id);
      if (id !== boundWorkspaceId)
        await current.workspace({ kind: "open", workspace_id: id });
    },
  });
  if (id === boundWorkspaceId) {
    activateWorkbench(true);
    return;
  }
  await showWorkspace(id, null, state ?? null);
}
async function openContentFile(
  current: Engine,
  job: OpenJob,
  state: WindowState | undefined,
): Promise<void> {
  const { input } = job;
  const file = await readImport(input, contract.file_bytes, "file_limit");
  if (input.toLowerCase().endsWith(".json"))
    await current.workspace({
      kind: "validate_challenge",
      package: file.toString("base64"),
    });
  let workspaceId = job.workspaceId ?? boundWorkspaceId;
  if (!workspaceId) {
    const created = await current.workspace({
      kind: "create",
      key: randomUUID(),
      title: "Local files",
    });
    workspaceId = created.id;
    boundWorkspaceId = workspaceId;
  }
  let importedLocation: OpenIntent | null = null;
  if (input.toLowerCase().endsWith(".json")) {
    const imported = (await current.workspace({
      kind: "import_challenge",
      workspace_id: workspaceId,
      key: randomUUID(),
      package: file.toString("base64"),
    })) as ImportedChallenge;
    importedLocation = {
      workspace_id: workspaceId,
      exploration_id: imported.explorationId,
      stage_id: imported.stageId,
    };
  } else {
    let context = job.context;
    if (!context || context.workspace_id !== workspaceId) {
      const created = await current.workspace({
        kind: "create_exploration",
        workspace_id: workspaceId,
        key: randomUUID(),
        title: basename(input).slice(0, 100),
      });
      context = {
        workspace_id: workspaceId,
        exploration_id: created.explorationId,
        stage_id: created.stageId,
      };
    }
    importedLocation = { ...context };
    const title = basename(input);
    if (input.toLowerCase().endsWith(".png"))
      await current.workspace({
        kind: "add_image",
        context,
        key: randomUUID(),
        title,
        caption: "",
        png: file.toString("base64"),
      });
    else
      await current.workspace({
        kind: "add_file",
        context,
        key: randomUUID(),
        title,
        base64: file.toString("base64"),
      });
  }
  if (state && !sameContext(state.context, job.context)) return;
  await showWorkspace(workspaceId, importedLocation, state ?? null);
}
async function openLocal(job: OpenJob): Promise<void> {
  const { input } = job;
  const current = engine;
  if (!current) throw new Error("engine_unavailable");
  const state = job.owner ?? activeWindow();
  if (
    state?.window.isDestroyed() ||
    (job.workspaceId && state?.workspaceId !== job.workspaceId)
  )
    throw new Error("stale_open_target");
  const wasDirty = state?.dirty ?? false;
  const documentEpoch = state?.documentEpoch;
  if (state?.dirty && !(await confirmDiscard(state)))
    throw new Error("open_cancelled");
  if (state) state.window.setEnabled(false);
  try {
    if (input.startsWith("cube-play-open:")) {
      await openWorkspaceLink(current, input, state);
    } else {
      if (!isAbsolute(input)) throw new Error("invalid_open_intent");
      if ((await stat(input)).isDirectory())
        await openSourceFolder(current, input, state);
      else await openContentFile(current, job, state);
    }
  } finally {
    if (state && !state.window.isDestroyed()) {
      state.window.setEnabled(true);
      if (wasDirty && state.documentEpoch === documentEpoch) {
        state.dirty = true;
        state.window.setDocumentEdited(true);
      }
    }
  }
}
function sameContext(a: Context | null, b: Context | null): boolean {
  return (
    a?.workspace_id === b?.workspace_id &&
    a?.exploration_id === b?.exploration_id &&
    a?.stage_id === b?.stage_id
  );
}
function queueOpen(
  input: string,
  owner: WindowState | null = activeWindow() ?? null,
) {
  if (quitting || input.length > 4096 || pendingOpens.length >= 8) return;
  if (
    !pendingOpens.some(
      (job) =>
        job.input === input &&
        job.owner === owner &&
        sameContext(job.context, owner?.context ?? null),
    )
  )
    pendingOpens.push({
      input,
      owner,
      workspaceId: owner?.workspaceId ?? null,
      context: owner?.context ? { ...owner.context } : null,
    });
  void drainOpens().catch(() => {});
}
async function drainOpens() {
  if (opening || !engine) return;
  opening = true;
  try {
    while (pendingOpens.length) {
      const job = pendingOpens.shift();
      if (!job || quitting) continue;
      try {
        if (switchingWorkspace) throw new Error("busy");
        switchingWorkspace = true;
        try {
          await openLocal(job);
        } finally {
          switchingWorkspace = false;
        }
      } catch (error) {
        if (error instanceof Error && error.message === "open_cancelled")
          continue;
        await dialog.showMessageBox({
          type: "error",
          message: "This item could not be opened.",
          detail:
            error instanceof Error && error.message === "workspace_in_use"
              ? "Close the other workbench windows before switching Workspaces."
              : "Use a supported local package, PNG, bounded file, source folder or Workspace link. Existing content was preserved.",
        });
      }
    }
  } finally {
    opening = false;
  }
}
function receiveArguments(argv: string[]): boolean {
  const index = argv.indexOf("--desktop-open");
  const path = index < 0 ? undefined : argv[index + 1];
  const intents = new Set<string>();
  if (path)
    intents.add(path.startsWith("cube-play-open:") ? path : resolve(path));
  for (const value of argv)
    if (value.startsWith("cube-play-open:")) intents.add(value);
  for (const input of intents) queueOpen(input);
  return intents.size > 0;
}
app.on("open-file", (event, path) => {
  event.preventDefault();
  queueOpen(path);
});
app.on("open-url", (event, url) => {
  event.preventDefault();
  queueOpen(url);
});
app.on("second-instance", (_event, argv) => {
  if (receiveArguments(argv)) activeWindow()?.window.focus();
  else activateWorkbench();
});
receiveArguments(process.argv);

app
  .whenReady()
  .then(async () => {
    const sandbox = session.defaultSession;
    sandbox.setPermissionRequestHandler((_contents, _permission, callback) =>
      callback(false),
    );
    sandbox.setPermissionCheckHandler(() => false);
    sandbox.webRequest.onBeforeRequest((details, callback) => {
      callback({
        cancel:
          resourceName(details.url) === null &&
          ![...windows.values()].some((state) =>
            state.grants.allows(details.url, details.webContentsId ?? -1),
          ),
      });
    });
    protocol.handle("cube-play", async (request) => {
      if (request.method === "GET") {
        const image = [...windows.values()]
          .map((state) => state.grants.read(request.url))
          .find((bytes) => bytes !== null);
        if (image)
          return new Response(new Uint8Array(image), {
            headers: {
              "Content-Type": "image/png",
              "Cache-Control": "no-store",
              "X-Content-Type-Options": "nosniff",
            },
          });
      }
      const name = resourceName(request.url);
      if (request.method !== "GET" || !name)
        return new Response(null, { status: 403 });
      const contentType = name.endsWith(".ttf")
        ? "font/ttf"
        : name.endsWith(".js")
          ? "text/javascript"
          : name.endsWith(".css")
            ? "text/css"
            : "text/html";
      const resource = await readFile(new URL(name, resources));
      const content =
        name === "index.html"
          ? resource
              .toString("utf8")
              .replace(
                "<head>",
                `<head><meta name="play-style-nonce" content="${styleNonce}">`,
              )
          : resource;
      return new Response(content, {
        headers: {
          "Content-Type": contentType,
          "Content-Security-Policy": `default-src 'none'; script-src 'self'; style-src 'self' 'nonce-${styleNonce}'; font-src 'self'; img-src 'self'; connect-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'; object-src 'none'`,
          "X-Content-Type-Options": "nosniff",
          "Cache-Control": "no-store",
        },
      });
    });
    await verifyNative(native);
    engine = await Engine.connect(
      enginePath,
      workerPath,
      join(profile, "workspaces"),
      files.terminal
        ? fileURLToPath(new URL(files.terminal, native))
        : undefined,
    );
    nativeExports = new NativeExports(engine, [profile, app.getAppPath()]);
    cli = new Cli(fileURLToPath(new URL(files.cli, native)), engine);
    const authorize = (event: Electron.IpcMainInvokeEvent): Engine => {
      windowFor(event);
      if (!engine) {
        throw new Error("unauthorized_frame");
      }
      return engine;
    };
    ipcMain.handle("play:location", (event) => {
      authorize(event);
      const state = windowFor(event);
      const location = state.initialLocation;
      state.initialLocation = null;
      return location;
    });
    ipcMain.handle("play:workspace", async (event, value: unknown) => {
      const engine = authorize(event);
      const command = request(value);
      const state = windowFor(event);
      if (
        windows.size > 1 &&
        (command.kind === "create" ||
          (command.kind === "open" &&
            boundWorkspaceId &&
            command.workspace_id !== boundWorkspaceId))
      )
        throw new Error("workspace_in_use");
      const switching =
        command.kind === "create" ||
        (command.kind === "open" && command.workspace_id !== boundWorkspaceId);
      if (
        switchingWorkspace &&
        (switching || command.kind === "open_exploration")
      )
        throw new Error("busy");
      const navigation =
        command.kind === "open_exploration"
          ? ++state.navigationEpoch
          : state.navigationEpoch;
      if (switching) switchingWorkspace = true;
      let data: unknown;
      try {
        data = await engine.workspace(command);
        if (command.kind === "create" || command.kind === "open") {
          const id = (data as { id: string }).id;
          if (state.workspaceId !== id) {
            state.context = null;
            state.navigationEpoch++;
            state.grants.select(null);
          }
          if (boundWorkspaceId !== id) await disconnectAttachments();
          state.workspaceId = boundWorkspaceId = id;
        }
      } finally {
        if (switching) switchingWorkspace = false;
      }
      if (
        command.kind === "open_exploration" &&
        navigation === state.navigationEpoch
      ) {
        const exploration = data as ExplorationView;
        state.context = {
          workspace_id: exploration.workspaceId,
          exploration_id: exploration.id,
          stage_id: exploration.stage.id,
        };
        state.grants.select({
          workspace_id: exploration.workspaceId,
          exploration_id: exploration.id,
          stage_id: exploration.stage.id,
        });
      }
      return data;
    });
    ipcMain.handle("play:attachment", async (event, value: unknown) => {
      const engine = authorize(event);
      const state = windowFor(event);
      const intent = attachmentIntent(value);
      if (intent.kind === "close") {
        await attachments.close(event.sender.id, intent.id);
        return null;
      }
      if (
        !state.context ||
        Object.entries(intent.context).some(
          ([key, value]) => state.context?.[key as keyof Context] !== value,
        )
      )
        throw new Error("stale_context");
      const { id, attachment } = await attachments.open(
        event.sender.id,
        intent.context,
        intent.selection,
        (value) => dispatchCli(engine, value),
      );
      return {
        id,
        prefix: contextCommand(
          fileURLToPath(new URL(files.cli, native)),
          attachment.path,
        ),
        expiresMs: Date.now() + 10 * 60_000,
      };
    });
    ipcMain.handle("play:cli", (event, value: unknown) => {
      authorize(event);
      if (!cli) throw new Error("cli_unavailable");
      return cli.execute(cliIntent(value));
    });
    ipcMain.handle("play:terminal", (event, value: unknown) =>
      authorize(event).terminal(terminalIntent(value)),
    );
    ipcMain.handle("play:start-run", (event, value: unknown) =>
      authorize(event).startRun(runRequest(value)),
    );
    ipcMain.handle("play:cancel-run", (event, value: unknown) =>
      authorize(event).cancelRun(cancelRun(value)),
    );
    ipcMain.handle("play:export", async (event, value: unknown) => {
      authorize(event);
      const state = windowFor(event);
      const intent = exportIntent(value);
      const frame = event.senderFrame;
      const navigation = state.navigationEpoch,
        document = state.documentEpoch;
      const current = () =>
        !quitting &&
        !state.window.isDestroyed() &&
        state.window.webContents.mainFrame === frame &&
        state.navigationEpoch === navigation &&
        state.documentEpoch === document &&
        Boolean(state.context) &&
        Object.entries(intent.context).every(
          ([key, value]) => state.context?.[key as keyof Context] === value,
        );
      if (!current()) throw new Error("stale_context");
      if (!nativeExports || exportingWindows.has(event.sender.id))
        throw new Error("busy");
      exportingWindows.add(event.sender.id);
      try {
        return await nativeExports.save(
          intent,
          async (suggestion) => {
            const selection = await dialog.showSaveDialog(state.window, {
              title:
                intent.source.kind === "solution"
                  ? "Export Solution walkthrough"
                  : "Export selected evidence",
              defaultPath: suggestion.fileName,
              message:
                "Choose a new filename. Existing files are never replaced.",
            });
            return selection.canceled || !selection.filePath
              ? null
              : selection.filePath;
          },
          current,
        );
      } finally {
        exportingWindows.delete(event.sender.id);
      }
    });
    ipcMain.handle("play:import-challenge", (event, value: unknown) => {
      const engine = authorize(event);
      return importChallenge(
        engine,
        windowFor(event).window,
        challengeImport(value),
      );
    });
    ipcMain.handle("play:import-image", (event, value: unknown) => {
      const engine = authorize(event);
      return importImage(engine, windowFor(event).window, imageImport(value));
    });
    ipcMain.handle("play:import-file", (event, value: unknown) => {
      const engine = authorize(event);
      return importFile(engine, windowFor(event).window, fileImport(value));
    });
    ipcMain.handle("play:image", (event, value: unknown) => {
      const engine = authorize(event);
      if (!event.senderFrame) throw new Error("unauthorized_frame");
      return windowFor(event).grants.issue(
        engine,
        event.sender,
        event.senderFrame,
        imageRead(value),
      );
    });
    ipcMain.handle("play:dirty", (event, value: unknown) => {
      authorize(event);
      if (typeof value !== "boolean") throw new Error("invalid_command");
      const state = windowFor(event);
      state.dirty = value;
      state.window.setDocumentEdited(value);
    });
    ipcMain.handle("play:catalog", (event) => authorize(event).catalog());
    Menu.setApplicationMenu(
      Menu.buildFromTemplate(
        menuTemplate(process.platform, {
          newWindow: () => createWindow(),
          copyLink: () => {
            const state = activeWindow();
            if (state?.workspaceId)
              clipboard.writeText(
                workspaceLink({
                  workspace_id: state.workspaceId,
                  exploration_id: state.context?.exploration_id ?? null,
                  stage_id: state.context?.stage_id ?? null,
                }),
              );
          },
          disconnect: () => void disconnectAttachments(),
          openFolder: () => {
            const state = activeWindow();
            const options: Electron.OpenDialogOptions = {
              properties: ["openDirectory"],
              title: "Open a source folder",
            };
            void (
              state
                ? dialog.showOpenDialog(state.window, options)
                : dialog.showOpenDialog(options)
            ).then((result) => {
              if (!result.canceled && result.filePaths[0])
                queueOpen(result.filePaths[0], state ?? null);
            });
          },
        }),
      ),
    );
    createWindow();
    await drainOpens();
  })
  .catch(async () => {
    await disconnectAttachments();
    await cli?.close();
    await engine?.close();
    app.exit(1);
  });

app.on("activate", () => activateWorkbench());
app.on("window-all-closed", () => {});
app.on("before-quit", (event) => {
  if (quitting) return;
  event.preventDefault();
  void (async () => {
    for (const state of windows.values())
      if (!(await confirmDiscard(state))) return false;
    return true;
  })().then(async (discard) => {
    if (!discard || quitting) return;
    quitting = true;
    await nativeExports?.settle();
    for (const state of windows.values()) state.grants.revoke();
    await disconnectAttachments();
    await cli?.close();
    await engine?.close();
    app.exit(0);
  });
});
