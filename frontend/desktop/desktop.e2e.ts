import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  type ElectronApplication,
  _electron as electron,
  expect,
  type Page,
} from "@playwright/test";
import type {} from "./bridge.ts";
import type { Context } from "./workspace-contract.ts";

const main = fileURLToPath(new URL("main.js", import.meta.url));
const fixture = fileURLToPath(
  new URL("../../tests/fixtures/sample.png", import.meta.url),
);
const notebook = (page: Page) =>
  page.getByRole("region", { name: "Stage notebook" });
const explorer = (page: Page) =>
  page.getByRole("complementary", { name: "Explorer" });

async function create(
  page: Page,
  kind: "Exploration" | "Stage",
  title: string,
) {
  await explorer(page)
    .getByRole("button", { name: `New ${kind}`, exact: true })
    .click();
  const dialog = page.getByRole("dialog", { name: `New ${kind}`, exact: true });
  await dialog.getByRole("textbox", { name: "Title", exact: true }).fill(title);
  await dialog.getByRole("button", { name: "Create", exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(notebook(page)).toBeVisible();
}
async function saveNote(page: Page, text: string) {
  await page.getByLabel("Stage note", { exact: true }).fill(text);
  await page.keyboard.press("ControlOrMeta+s");
  await expect(
    page.getByRole("status", { name: "Note save status" }),
  ).toHaveText("Saved");
}
async function capture(application: ElectronApplication, name: string) {
  const png = await application.evaluate(async ({ BrowserWindow }) => {
    const window = BrowserWindow.getAllWindows().find((entry) =>
      entry.isVisible(),
    );
    if (!window) throw new Error("visible_window_missing");
    return (await window.webContents.capturePage()).toPNG().toString("base64");
  });
  await writeFile(`playwright-report/${name}.png`, Buffer.from(png, "base64"));
}
async function checkWorkbench(page: Page) {
  await expect(explorer(page)).toBeVisible();
  await page.keyboard.press("ControlOrMeta+b");
  await expect(explorer(page)).toBeHidden();
  await page.keyboard.press("ControlOrMeta+b");
  await expect(explorer(page)).toBeVisible();
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
  const widthBefore = (await explorer(page).boundingBox())?.width;
  const sash = await page
    .locator(".workspace-split .handle.split-vertical")
    .boundingBox();
  assert.ok(widthBefore && sash);
  await page.mouse.move(sash.x + sash.width / 2, sash.y + 150);
  await page.mouse.down();
  await page.mouse.move(sash.x + 42, sash.y + 150, { steps: 5 });
  await page.mouse.up();
  await expect
    .poll(async () => (await explorer(page).boundingBox())?.width ?? 0)
    .toBeGreaterThan(widthBefore + 20);
}
async function current(page: Page) {
  return page.evaluate(async () => {
    const list = await window.play.workspace({ kind: "list" });
    if (!list.lastWorkspaceId) throw new Error("workspace_missing");
    const workspace = await window.play.workspace({
      kind: "open",
      workspace_id: list.lastWorkspaceId,
    });
    if (!workspace.selectedExplorationId)
      throw new Error("exploration_missing");
    const exploration = await window.play.workspace({
      kind: "open_exploration",
      workspace_id: workspace.id,
      exploration_id: workspace.selectedExplorationId,
      stage_id: workspace.selectedStageId,
    });
    return { workspace, exploration };
  });
}
async function checkAuthority(
  application: ElectronApplication,
  page: Page,
  context: Context,
) {
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
  const imageUrl = await notebook(page)
    .getByRole("img", { name: "sample.png", exact: true })
    .getAttribute("src");
  assert.match(imageUrl ?? "", /^cube-play:\/\/app\/assets\/[0-9a-f]{32}$/);
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
  const denied = await rogue.evaluate(async (url) => {
    const results = [];
    for (const invoke of [
      () => window.play.catalog(),
      () => window.play.workspace({ kind: "list" }),
    ]) {
      try {
        await invoke();
        results.push(false);
      } catch {
        results.push(true);
      }
    }
    results.push(
      await new Promise<boolean>((resolve) => {
        const image = new Image();
        image.onload = () => resolve(false);
        image.onerror = () => resolve(true);
        image.src = url ?? "";
      }),
    );
    return results;
  }, imageUrl);
  assert.deepEqual(
    denied,
    [true, true, true],
    "matching URLs cannot authorize another window or its image requests",
  );
  await rogue.close();
  const invalid = await page.evaluate(async (context) => {
    const values = [
      { kind: "open", workspace_id: "../../source" },
      { kind: "list", path: "/private/file" },
      { kind: "add_image", context, png: "untrusted" },
      {
        kind: "add_text",
        context,
        key: crypto.randomUUID(),
        title: "too large",
        text: "é".repeat(513),
      },
    ];
    return Promise.all(
      values.map(async (value) => {
        try {
          await window.play.workspace(
            value as Parameters<typeof window.play.workspace>[0],
          );
          return false;
        } catch {
          return true;
        }
      }),
    );
  }, context);
  assert.deepEqual(invalid, [true, true, true, true]);
}

test("native workspaces persist independent notebooks, owned assets and reasoning across a real restart", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-electron-"));
  const sourceBefore = await readFile(fixture);
  const launch = () =>
    electron.launch({ args: [main, `--user-data-dir=${profile}`] });
  let application = await launch();
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    const welcome = page.getByRole("region", { name: "Workspace welcome" });
    await expect(welcome).toBeVisible({ timeout: 3000 });
    await welcome
      .getByRole("textbox", { name: "Workspace name" })
      .fill("Signal studies");
    await welcome
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await expect(
      page.getByRole("region", { name: "Empty workspace" }),
    ).toBeVisible();
    await create(page, "Exploration", "Spiral signal");
    await mkdir("playwright-report", { recursive: true });
    await checkWorkbench(page);
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, fixture);
    await notebook(page)
      .getByRole("button", { name: "Add image", exact: true })
      .click();
    const imageDialog = page.getByRole("dialog", {
      name: "Add image with caption",
    });
    await imageDialog
      .getByRole("textbox", { name: "Caption", exact: true })
      .fill("Gur xrl vf va gur cvkryf.");
    await imageDialog
      .getByRole("button", { name: "Choose PNG", exact: true })
      .click();
    await expect(imageDialog).toBeHidden();
    const imageBlock = notebook(page).getByRole("article", {
      name: "sample.png block",
      exact: true,
    });
    await expect(imageBlock.getByRole("figure")).toContainText(
      "Gur xrl vf va gur cvkryf.",
    );
    await expect(
      imageBlock.getByRole("img", { name: "sample.png", exact: true }),
    ).toBeVisible();
    await expect(
      notebook(page).getByRole("article", { name: "Image caption block" }),
    ).toHaveCount(0);
    await expect(
      page.getByRole("region", { name: "Output", exact: true }),
    ).toHaveCount(0);
    await saveNote(page, "The caption belongs with the image.");
    await expect(page.getByRole("combobox", { name: "Operation" })).toHaveText(
      "ROT13",
    );
    await imageBlock
      .getByRole("button", { name: "Run cell", exact: true })
      .click();
    const result =
      process.platform === "darwin"
        ? "The key is in the pixels."
        : "Execution rejected";
    await expect(
      imageBlock.getByRole("region", { name: "Cell result" }),
    ).toContainText(result);
    await capture(application, "native-notebook-result");
    const first = await current(page);
    const firstContext = {
      workspace_id: first.workspace.id,
      exploration_id: first.exploration.id,
      stage_id: first.exploration.stage.id,
    };
    await checkAuthority(application, page, firstContext);
    await imageBlock
      .getByRole("button", { name: "Explore Image caption", exact: true })
      .click();
    const canvas = page.getByRole("region", { name: "Trail canvas" });
    await expect(canvas).toBeVisible();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(result);
    await page.keyboard.press("ControlOrMeta+Enter");
    await expect(canvas.getByText("Out [2]", { exact: true })).toBeVisible();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(result);
    await capture(application, "native-caption-canvas");
    await create(page, "Exploration", "Alternative hypothesis");
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "",
    );
    await expect(
      notebook(page).getByRole("region", { name: "Cell result" }),
    ).toHaveCount(0);
    await expect(
      notebook(page).getByRole("article", { name: "sample.png block" }),
    ).toHaveCount(0);
    await saveNote(page, "Independent notes.");
    await notebook(page)
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    const textDialog = page.getByRole("dialog", { name: "Add text block" });
    await textDialog
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Second clue");
    await textDialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("frpbaq");
    await textDialog
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    await expect(textDialog).toBeHidden();
    const second = await current(page);
    assert.equal(
      second.workspace.pipelines.length,
      1,
      "trails reuse the workspace pipeline",
    );
    const wrongOwner = await page.evaluate(
      async ({ context, assetId }) => {
        try {
          await window.play.runAsset({
            context,
            asset_id: assetId,
            revision: 1,
            operation: "cipher.rot13",
            operation_version: 1,
          });
          return false;
        } catch {
          return true;
        }
      },
      {
        context: {
          workspace_id: first.workspace.id,
          exploration_id: second.exploration.id,
          stage_id: second.exploration.stage.id,
        },
        assetId: first.exploration.stage.blocks[0]?.caption?.id ?? "",
      },
    );
    assert.equal(wrongOwner, true);
    await create(page, "Stage", "Follow-up");
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "",
    );
    await saveNote(page, "A separate local stage.");
    await explorer(page)
      .getByRole("button", { name: "Spiral signal", exact: true })
      .click();
    await expect(canvas).toBeVisible();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(result);
    await page.getByRole("tab", { name: "Stage 1", exact: true }).click();
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "The caption belongs with the image.",
    );
    await notebook(page)
      .getByRole("button", { name: "Explore sample.png", exact: true })
      .click();
    const image = canvas.getByRole("img", { name: "sample.png", exact: true });
    const width = (await image.boundingBox())?.width;
    assert.ok(width);
    await canvas.getByRole("button", { name: "Zoom in", exact: true }).click();
    await expect
      .poll(async () => (await image.boundingBox())?.width ?? 0)
      .toBeGreaterThan(width * 1.2);
    await expect(canvas.getByRole("button", { name: "Run cell" })).toHaveCount(
      0,
    );
    await capture(application, "native-image-canvas");
    await page
      .getByRole("button", { name: "Close sample.png canvas", exact: true })
      .click();
    await capture(application, "native-desktop");
    await application.close();
    application = await launch();
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(notebook(page)).toBeVisible({ timeout: 3000 });
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "The caption belongs with the image.",
    );
    const restored = await current(page);
    assert.equal(restored.workspace.id, first.workspace.id);
    assert.equal(restored.exploration.id, first.exploration.id);
    assert.equal(
      restored.exploration.stage.blocks[0]?.asset.id,
      first.exploration.stage.blocks[0]?.asset.id,
    );
    await expect(
      notebook(page).getByRole("img", { name: "sample.png", exact: true }),
    ).toBeVisible();
    await expect(
      notebook(page).getByRole("region", { name: "Cell result" }),
    ).toHaveCount(0, { timeout: 1000 });
    await explorer(page)
      .getByRole("button", { name: "Alternative hypothesis", exact: true })
      .click();
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "A separate local stage.",
    );
    await explorer(page)
      .getByRole("button", { name: "01 · Stage 1", exact: true })
      .click();
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Independent notes.",
    );
    await expect(
      notebook(page).getByRole("article", {
        name: "Second clue block",
        exact: true,
      }),
    ).toContainText("frpbaq");
    assert.deepEqual(
      await readFile(fixture),
      sourceBefore,
      "the native import never modifies its source",
    );
    await capture(application, "native-restored-workspace");
  } finally {
    await application
      .evaluate(({ dialog }) => {
        dialog.showMessageBox = async () => ({
          response: 1,
          checkboxChecked: false,
        });
      })
      .catch(() => {});
    await application.close();
    await rm(profile, { recursive: true, force: true });
  }
});
