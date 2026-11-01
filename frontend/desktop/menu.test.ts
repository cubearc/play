import assert from "node:assert/strict";
import test from "node:test";
import { menuTemplate } from "./menu.ts";

test("desktop menus offer an explicit Quit action even when the last window stays open", () => {
  const actions = {
    newWindow() {},
    copyLink() {},
    disconnect() {},
    openFolder() {},
  };
  for (const platform of ["linux", "win32", "darwin"] as const) {
    const menu = menuTemplate(platform, actions);
    if (platform === "darwin")
      assert.ok(menu.some((item) => item.role === "appMenu"));
    else {
      const file = menu.find((item) => item.label === "File");
      assert.ok(Array.isArray(file?.submenu));
      assert.ok(
        file.submenu.some((item) => item.role === "quit"),
        `${platform}: File must offer Quit`,
      );
    }
  }
});
