import { mkdirSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { isAbsolute, join } from "node:path";
import { fileURLToPath } from "node:url";
import { app, BrowserWindow, ipcMain, protocol, session } from "electron";
import { APP_URL, ownsFrame, resourceName } from "./authority.ts";
import { invocation } from "./contract.ts";
import { Engine } from "./engine.ts";
import { verifyNative } from "./integrity.ts";

protocol.registerSchemesAsPrivileged([
  {
    scheme: "cube-play",
    privileges: { standard: true, secure: true, supportFetchAPI: true },
  },
]);
app.enableSandbox();
app.setName("Cube Play Boundary Proof");
const profile =
  app.commandLine.getSwitchValue("user-data-dir") ||
  join(app.getPath("appData"), "CubeArc", "PlayBoundaryProof");
if (!isAbsolute(profile)) throw new Error("invalid_profile");
mkdirSync(profile, { recursive: true, mode: 0o700 });
app.setPath("userData", profile);

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
let window: BrowserWindow | undefined;
let quitting = false;

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
    title: "Cube Play — Native boundary proof",
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
  next.on("closed", () => {
    if (window === next) window = undefined;
    void engine?.cancel().catch(() => {});
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
      callback({ cancel: resourceName(details.url) === null });
    });
    protocol.handle("cube-play", async (request) => {
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
      return new Response(await readFile(new URL(name, resources)), {
        headers: {
          "Content-Type": contentType,
          "Content-Security-Policy":
            "default-src 'none'; script-src 'self'; style-src 'self'; font-src 'self'; connect-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'; object-src 'none'",
          "X-Content-Type-Options": "nosniff",
          "Cache-Control": "no-store",
        },
      });
    });
    await verifyNative(native);
    engine = await Engine.connect(enginePath, workerPath);
    const authorize = (event: Electron.IpcMainInvokeEvent): Engine => {
      if (!window || !ownsFrame(event, window.webContents) || !engine) {
        throw new Error("unauthorized_frame");
      }
      return engine;
    };
    ipcMain.handle("play:run", (event, value: unknown) =>
      authorize(event).run(invocation(value)),
    );
    ipcMain.handle("play:catalog", (event) => authorize(event).catalog());
    ipcMain.handle("play:cancel", (event) => authorize(event).cancel());
    createWindow();
  })
  .catch(async () => {
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
  quitting = true;
  void engine?.close().finally(() => app.exit(0));
  if (!engine) app.exit(0);
});
