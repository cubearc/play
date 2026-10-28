import { contextBridge, ipcRenderer } from "electron";
import contract from "../../desktop/protocol.json" with { type: "json" };

if (process.isMainFrame) {
  contextBridge.exposeInMainWorld("play", {
    catalog: () => ipcRenderer.invoke("play:catalog"),
    run: (operation: unknown, version: unknown, input: unknown) => {
      if (
        typeof operation !== "string" ||
        operation.length > 128 ||
        typeof version !== "number" ||
        !Number.isSafeInteger(version) ||
        version < 1 ||
        version > 0xffffffff ||
        typeof input !== "string" ||
        input.length > contract.input_bytes ||
        new TextEncoder().encode(input).byteLength > contract.input_bytes
      ) {
        return Promise.reject(new Error("invalid_command"));
      }
      return ipcRenderer.invoke("play:run", {
        operation,
        operation_version: version,
        input,
      });
    },
    cancel: () => ipcRenderer.invoke("play:cancel"),
  });
}
