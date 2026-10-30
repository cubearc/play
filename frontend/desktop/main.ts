import { randomBytes } from "node:crypto";
import { mkdirSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { isAbsolute, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  app,
  BrowserWindow,
  dialog,
  ipcMain,
  protocol,
  session,
} from "electron";
import { AssetGrants, importFile, importImage } from "./assets.ts";
import { APP_URL, ownsFrame, resourceName } from "./authority.ts";
import { Cli } from "./cli.ts";
import { cliIntent } from "./cli-contract.ts";
import { Engine } from "./engine.ts";
import { verifyNative } from "./integrity.ts";
import { terminalIntent } from "./terminal-contract.ts";
import {
  assetRead,
  cancelRun,
  type ExplorationView,
  fileImport,
  imageImport,
  request,
  runRequest,
} from "./workspace-contract.ts";

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

const styleNonce = randomBytes(24).toString("base64");
const resources = new URL("../desktop-renderer/", import.meta.url);
const native = new URL("../desktop-native/", import.meta.url);
const enginePath = fileURLToPath(
  new URL(
    process.platform === "win32" ? "play-engine.exe" : "play-engine",
    native,
  ),
);
const workerPath =
  process.platform === "darwin"
    ? fileURLToPath(new URL("Worker.app/Contents/MacOS/play-engine", native))
    : undefined;
let engine: Engine | undefined;
let cli: Cli | undefined;
let window: BrowserWindow | undefined;
let quitting = false;
let dirty = false;
let confirming = false;
let boundWorkspaceId: string | null = null;
const grants = new AssetGrants();
if (!app.requestSingleInstanceLock()) app.exit(0);
async function confirmDiscard(): Promise<boolean> {
  if (!dirty) return true;
  if (!window || confirming) return false;
  confirming = true;
  try {
    const result = await dialog.showMessageBox(window, {
      type: "warning",
      message: "Changes have not been saved.",
      detail:
        "Keep editing to save your content, or discard the unsaved draft.",
      buttons: ["Keep editing", "Discard draft"],
      defaultId: 0,
      cancelId: 0,
    });
    if (result.response !== 1) return false;
    dirty = false;
    window.setDocumentEdited(false);
    return true;
  } finally {
    confirming = false;
  }
}

function createWindow(): void {
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
  window = next;
  next.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  next.webContents.on("will-navigate", (event) => event.preventDefault());
  next.webContents.on("will-frame-navigate", (event) => event.preventDefault());
  next.webContents.on("will-attach-webview", (event) => event.preventDefault());
  next.on("close", (event) => {
    if (!dirty) return;
    event.preventDefault();
    void confirmDiscard().then((discard) => {
      if (discard && !next.isDestroyed()) next.close();
    });
  });
  next.on("closed", () => {
    grants.revoke();
    if (window === next) window = undefined;
  });
  void next.loadURL(APP_URL);
}

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
          !grants.allows(details.url, details.webContentsId ?? -1),
      });
    });
    protocol.handle("cube-play", async (request) => {
      if (request.method === "GET") {
        const image = grants.read(request.url);
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
      process.platform === "darwin"
        ? fileURLToPath(
            new URL("Terminal.app/Contents/MacOS/play-engine", native),
          )
        : undefined,
    );
    cli = new Cli(
      fileURLToPath(
        new URL(process.platform === "win32" ? "cube.exe" : "cube", native),
      ),
      engine,
    );
    const authorize = (event: Electron.IpcMainInvokeEvent): Engine => {
      if (!window || !ownsFrame(event, window.webContents) || !engine) {
        throw new Error("unauthorized_frame");
      }
      return engine;
    };
    ipcMain.handle("play:workspace", async (event, value: unknown) => {
      const engine = authorize(event);
      const command = request(value);
      if (
        command.kind === "create" ||
        (command.kind === "open" && command.workspace_id !== boundWorkspaceId)
      )
        grants.select(null);
      const data = await engine.workspace(command);
      if (command.kind === "create" || command.kind === "open")
        boundWorkspaceId = (data as { id: string }).id;
      if (command.kind === "open_exploration") {
        const exploration = data as ExplorationView;
        grants.select({
          workspace_id: exploration.workspaceId,
          exploration_id: exploration.id,
          stage_id: exploration.stage.id,
        });
      }
      return data;
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
    ipcMain.handle("play:import-image", (event, value: unknown) => {
      const engine = authorize(event);
      if (!window) throw new Error("unauthorized_frame");
      return importImage(engine, window, imageImport(value));
    });
    ipcMain.handle("play:import-file", (event, value: unknown) => {
      const engine = authorize(event);
      if (!window) throw new Error("unauthorized_frame");
      return importFile(engine, window, fileImport(value));
    });
    ipcMain.handle("play:image", (event, value: unknown) => {
      const engine = authorize(event);
      if (!event.senderFrame) throw new Error("unauthorized_frame");
      return grants.issue(
        engine,
        event.sender,
        event.senderFrame,
        assetRead(value),
      );
    });
    ipcMain.handle("play:dirty", (event, value: unknown) => {
      authorize(event);
      if (typeof value !== "boolean") throw new Error("invalid_command");
      dirty = value;
      window?.setDocumentEdited(value);
    });
    ipcMain.handle("play:catalog", (event) => authorize(event).catalog());
    createWindow();
  })
  .catch(async () => {
    await cli?.close();
    await engine?.close();
    app.exit(1);
  });

app.on("activate", () => {
  if (!window && engine && !quitting) createWindow();
});
app.on("window-all-closed", () => {});
app.on("before-quit", (event) => {
  if (quitting) return;
  event.preventDefault();
  void confirmDiscard().then(async (discard) => {
    if (!discard || quitting) return;
    quitting = true;
    grants.revoke();
    await cli?.close();
    await engine?.close();
    app.exit(0);
  });
});
