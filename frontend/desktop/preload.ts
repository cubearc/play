import { contextBridge, ipcRenderer } from "electron";
import type { PlayBridge } from "./bridge.ts";
import {
  assetRead,
  cancelRun,
  imageImport,
  request,
  runRequest,
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
    startRun: (value: unknown) =>
      ipcRenderer.invoke("play:start-run", runRequest(value)),
    cancelRun: (value: unknown) =>
      ipcRenderer.invoke("play:cancel-run", cancelRun(value)),
    dirty: (value: unknown) => {
      if (typeof value !== "boolean")
        return Promise.reject(new Error("invalid_command"));
      return ipcRenderer.invoke("play:dirty", value);
    },
  };
  contextBridge.exposeInMainWorld("play", bridge);
}
