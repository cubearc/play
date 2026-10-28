import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { _electron as electron, expect } from "@playwright/test";

test("real Electron renderer invokes Core and rejects another window's bridge", {
  timeout: 60000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-electron-"));
  const application = await electron.launch({
    args: [
      fileURLToPath(new URL("main.js", import.meta.url)),
      `--user-data-dir=${profile}`,
    ],
  });
  try {
    const page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page.getByRole("button", { name: "Run", exact: true }).waitFor();
    assert.equal(
      await page.evaluate(
        () => typeof (window as unknown as Record<string, unknown>).require,
      ),
      "undefined",
    );
    assert.equal(
      await page.evaluate(
        () => typeof (window as unknown as Record<string, unknown>).process,
      ),
      "undefined",
    );
    const explorer = page.getByRole("complementary", { name: "Explorer" });
    await explorer.waitFor({ timeout: 3000 });
    await page.keyboard.press("ControlOrMeta+b");
    await expect(explorer).toBeHidden();
    await page.keyboard.press("ControlOrMeta+b");
    await expect(explorer).toBeVisible();
    const output = page.getByRole("region", { name: "Output" });
    await page.keyboard.press("ControlOrMeta+j");
    await expect(output).toBeHidden();
    await page.keyboard.press("ControlOrMeta+j");
    await expect(output).toBeVisible();
    await page.keyboard.press("ControlOrMeta+Shift+p");
    const palette = page.getByRole("dialog", { name: "Command Palette" });
    await expect(palette).toBeVisible();
    await page
      .getByRole("textbox", { name: "Command", exact: true })
      .fill("Output");
    await page.keyboard.press("Enter");
    await expect(palette).toBeHidden();
    await expect(output).toBeHidden();
    await page.keyboard.press("ControlOrMeta+j");
    await expect(output).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    assert.ok(
      await page.evaluate(() =>
        Array.from(document.fonts).some(
          (face) => face.family === "codicon" && face.status === "loaded",
        ),
      ),
    );
    const widthBefore = (await explorer.boundingBox())?.width;
    const sash = await page
      .locator(".workspace-split .handle.split-vertical")
      .boundingBox();
    assert.ok(widthBefore && sash);
    await page.mouse.move(sash.x + sash.width / 2, sash.y + 150);
    await page.mouse.down();
    await page.mouse.move(sash.x + 42, sash.y + 150, { steps: 5 });
    await page.mouse.up();
    await expect
      .poll(async () => (await explorer.boundingBox())?.width ?? 0)
      .toBeGreaterThan(widthBefore + 20);
    assert.ok(
      await page.evaluate(
        () => customElements.get("vscode-button") !== undefined,
      ),
    );
    await page.getByRole("combobox", { name: "Operation" }).click();
    await page
      .getByRole("option", { name: "cipher.rot13", exact: true })
      .click();
    await page.getByLabel("Input").fill("Hello, CubeArc!");
    await page.getByRole("button", { name: "Run", exact: true }).click();
    if (process.platform === "darwin") {
      await page
        .getByRole("status")
        .filter({ hasText: "Uryyb, PhorNep!" })
        .waitFor();
      await page.keyboard.press("ControlOrMeta+j");
      await expect(output).toBeHidden();
      await page.keyboard.press("ControlOrMeta+Enter");
      await expect(output).toBeVisible();
      await page
        .getByRole("status")
        .filter({ hasText: "Uryyb, PhorNep!" })
        .waitFor();
      const cancellation = await page.evaluate(async () => {
        const bridge = (
          window as unknown as {
            play: {
              run: (
                operation: string,
                version: number,
                input: string,
              ) => Promise<{ status: string }>;
              cancel: () => Promise<void>;
            };
          }
        ).play;
        const run = bridge.run("esolang.brainfuck.execute", 2, "+[+]");
        await bridge.cancel();
        return (await run).status;
      });
      assert.equal(cancellation, "cancelled");
    } else {
      await page
        .getByRole("status")
        .filter({ hasText: "Execution rejected" })
        .waitFor();
    }
    const rogueWindow = application.waitForEvent("window");
    await application.evaluate(
      ({ BrowserWindow }, preload) => {
        const rogue = new BrowserWindow({
          show: false,
          webPreferences: {
            preload,
            sandbox: true,
            contextIsolation: true,
            nodeIntegration: false,
          },
        });
        void rogue.loadURL("cube-play://app/index.html");
      },
      fileURLToPath(new URL("preload.cjs", import.meta.url)),
    );
    const rogue = await rogueWindow;
    await rogue.waitForLoadState();
    const denied = await rogue.evaluate(async () => {
      const bridge = (
        window as unknown as { play: { catalog: () => Promise<unknown> } }
      ).play;
      try {
        await bridge.catalog();
        return false;
      } catch {
        return true;
      }
    });
    assert.equal(
      denied,
      true,
      "loading the same URL must not authorize a different window",
    );
    await rogue.close();
    const invalidInput = await page.evaluate(async () => {
      const bridge = (
        window as unknown as {
          play: {
            run: (
              op: string,
              version: number,
              input: string,
            ) => Promise<unknown>;
          };
        }
      ).play;
      try {
        await bridge.run("cipher.rot13", 1, "x".repeat(8193));
        return false;
      } catch {
        return true;
      }
    });
    assert.equal(invalidInput, true);
    await mkdir("playwright-report", { recursive: true });
    await page.screenshot({ path: "playwright-report/native-desktop.png" });
  } finally {
    await application.close();
    await rm(profile, { recursive: true, force: true });
  }
});
