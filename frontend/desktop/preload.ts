import { contextBridge, ipcRenderer } from "electron";
import type { PlayBridge } from "./bridge.ts";
import {
  assetRead,
  assetRun,
  imageImport,
  request,
} from "./workspace-contract.ts";

if (process.isMainFrame) {
  const bridge: PlayBridge = {
    catalog: () => ipcRenderer.invoke("play:catalog"),
    workspace: (value: unknown) =>
      ipcRenderer.invoke("play:workspace", request(value)),
    importImage: (value: unknown) =>
      ipcRenderer.invoke("play:import-image", imageImport(value)),
    image: (value: unknown) =>
      ipcRenderer.invoke("play:image", assetRead(value)),
    runAsset: (value: unknown) =>
      ipcRenderer.invoke("play:run-asset", assetRun(value)),
    dirty: (value: unknown) => {
      if (typeof value !== "boolean")
        return Promise.reject(new Error("invalid_command"));
      return ipcRenderer.invoke("play:dirty", value);
    },
    cancel: () => ipcRenderer.invoke("play:cancel"),
  };
  contextBridge.exposeInMainWorld("play", bridge);
}
