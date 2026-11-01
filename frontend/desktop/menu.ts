import type { MenuItemConstructorOptions } from "electron";

export interface MenuActions {
  newWindow: () => void;
  copyLink: () => void;
  disconnect: () => void;
  openFolder: () => void;
}
export function menuTemplate(
  platform: NodeJS.Platform,
  actions: MenuActions,
): MenuItemConstructorOptions[] {
  return [
    ...(platform === "darwin" ? [{ role: "appMenu" as const }] : []),
    {
      label: "File",
      submenu: [
        {
          label: "New Window",
          accelerator: "CmdOrCtrl+Shift+N",
          click: actions.newWindow,
        },
        { label: "Copy Workspace Link", click: actions.copyLink },
        { label: "Disconnect External Terminals", click: actions.disconnect },
        {
          label: "Open Folder…",
          accelerator: "CmdOrCtrl+O",
          click: actions.openFolder,
        },
        { role: "close" },
        ...(platform === "darwin" ? [] : [{ role: "quit" as const }]),
      ],
    },
    { role: "editMenu" },
    { role: "viewMenu" },
    { role: "windowMenu" },
  ];
}
