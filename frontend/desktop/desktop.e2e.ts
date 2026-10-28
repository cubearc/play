import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { _electron as electron, expect, type Page } from "@playwright/test";

async function checkWorkbench(page: Page) {
  const explorer = page.getByRole("complementary", { name: "Explorer" });
  await expect(explorer).toBeVisible();
  await page.keyboard.press("ControlOrMeta+b");
  await expect(explorer).toBeHidden();
  await page.keyboard.press("ControlOrMeta+b");
  await expect(explorer).toBeVisible();
  await page.keyboard.press("ControlOrMeta+Shift+p");
  const palette = page.getByRole("dialog", { name: "Command Palette" });
  await expect(palette).toBeVisible();
  await page
    .getByRole("textbox", { name: "Command", exact: true })
    .fill("Stage notebook");
  await page.keyboard.press("Enter");
  await expect(palette).toBeHidden();
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
}

async function checkNotebook(
  page: Page,
  capture: (name: string) => Promise<void>,
) {
  const notebook = page.getByRole("region", { name: "Stage notebook" });
  await expect(notebook).toBeVisible();
  await expect(
    page.getByRole("region", { name: "Output", exact: true }),
  ).toHaveCount(0);
  await expect(
    notebook.getByRole("article", { name: "signal.png block" }),
  ).toBeVisible();
  await expect(
    notebook.getByRole("article", { name: "Image caption block" }),
  ).toBeVisible();
  await page
    .getByLabel("Stage note", { exact: true })
    .fill("The caption is evidence, separate from the image.");
  await page.getByRole("button", { name: "Run cell", exact: true }).click();
  const expected =
    process.platform === "darwin"
      ? "The key is in the pixels."
      : "Execution rejected";
  await expect(notebook.getByRole("status")).toContainText(expected);
  await expect(
    notebook
      .getByRole("article", { name: "Image caption block" })
      .getByRole("status"),
  ).toContainText(expected);
  await notebook
    .getByRole("article", { name: "Image caption block" })
    .scrollIntoViewIfNeeded();
  await capture("native-notebook-result");
  await page
    .getByRole("button", { name: "Explore Image caption", exact: true })
    .click();
  const canvas = page.getByRole("region", { name: "Trail canvas" });
  await expect(canvas).toBeVisible();
  await expect(
    page.getByRole("tab", { name: "Image caption", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(
    canvas.getByRole("article", { name: "Bound source asset" }),
  ).toContainText("signal-caption @ r1");
  await expect(canvas.getByRole("status")).toContainText(expected);
  await page.keyboard.press("ControlOrMeta+Enter");
  await expect(canvas.getByText("Out [2]", { exact: true })).toBeVisible();
  await expect(canvas.getByRole("status")).toContainText(expected);
  await page.screenshot({
    path: "playwright-report/native-caption-canvas.png",
  });
  await page
    .getByRole("button", { name: "Alternative hypothesis", exact: true })
    .click();
  await expect(notebook).toBeVisible();
  await expect(notebook.getByRole("status")).toHaveCount(0);
  await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue("");
  await page
    .getByLabel("Stage note", { exact: true })
    .fill("Independent notes.");
  await page.getByRole("combobox", { name: "Operation" }).click();
  await page
    .getByRole("option", { name: "esolang.brainfuck.execute", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Spiral signal", exact: true })
    .click();
  await expect(canvas).toBeVisible();
  await expect(canvas.getByRole("status")).toContainText(expected);
  await expect(page.getByRole("combobox", { name: "Operation" })).toHaveText(
    "cipher.rot13",
  );
  await page
    .getByRole("tab", { name: "Image and caption", exact: true })
    .click();
  await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
    "The caption is evidence, separate from the image.",
  );
  await page
    .getByRole("button", { name: "Explore signal.png", exact: true })
    .click();
  await expect(
    canvas.getByRole("article", { name: "Bound source asset" }),
  ).toContainText("signal-image @ r1");
  const beforeZoom = await canvas
    .getByRole("img", { name: "Illustrative pixel grid" })
    .boundingBox();
  assert.ok(beforeZoom);
  await page.getByRole("button", { name: "Zoom in", exact: true }).click();
  const afterZoom = await canvas
    .getByRole("img", { name: "Illustrative pixel grid" })
    .boundingBox();
  assert.ok(afterZoom && afterZoom.width > beforeZoom.width * 1.2);
  await expect(page.getByLabel("Image zoom", { exact: true })).toHaveText(
    "125%",
  );
  await expect(canvas.getByRole("button", { name: "Run cell" })).toHaveCount(0);
  await capture("native-image-canvas");
  await page
    .getByRole("button", { name: "Close signal.png canvas", exact: true })
    .click();
  await expect(notebook).toBeVisible();
  await expect(notebook.getByRole("status")).toContainText(expected);
  await notebook.evaluate((element) => {
    element.scrollTop = 0;
  });
  await capture("native-desktop");
}

test("native notebook keeps evidence scoped and opens asset trails without a global output panel", {
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
    await page
      .getByRole("region", { name: "Stage notebook" })
      .waitFor({ timeout: 3000 });
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
    await mkdir("playwright-report", { recursive: true });
    await checkWorkbench(page);
    await checkNotebook(page, async (name) => {
      const png = await application.evaluate(async ({ BrowserWindow }) => {
        const window = BrowserWindow.getAllWindows().find((entry) =>
          entry.isVisible(),
        );
        if (!window) throw new Error("visible_window_missing");
        return (await window.webContents.capturePage())
          .toPNG()
          .toString("base64");
      });
      await writeFile(
        `playwright-report/${name}.png`,
        Buffer.from(png, "base64"),
      );
    });
    if (process.platform === "darwin") {
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
      "the same URL must not authorize another window",
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
  } finally {
    await application.close();
    await rm(profile, { recursive: true, force: true });
  }
});
