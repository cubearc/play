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
import { type Context, reasoningOf } from "./workspace-contract.ts";

test("annotation Blocks save independently, preserve drafts and restore their Note identity", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-annotations-"));
  const launch = () =>
    electron.launch({ args: [main, `--user-data-dir=${profile}`] });
  let application = await launch();
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Notebook annotations");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Observations");
    const add = async (title: string, text: string) => {
      await notebook(page)
        .getByRole("button", { name: "Add annotation", exact: true })
        .click();
      const dialog = page.getByRole("dialog", {
        name: "Add annotation",
        exact: true,
      });
      await dialog
        .getByRole("textbox", { name: "Title", exact: true })
        .fill(title);
      await dialog
        .getByRole("textbox", { name: "Text", exact: true })
        .fill(text);
      await dialog
        .getByRole("button", { name: "Add annotation", exact: true })
        .click();
      await expect(dialog).toBeHidden();
      await expect(
        notebook(page).getByRole("article", {
          name: `${title} block`,
          exact: true,
        }),
      ).toBeVisible();
    };
    await add("Observation", "The caption may be encoded.");
    await add("Alternative", "Inspect the pixels first.");
    const cell = (title: string) =>
      notebook(page).getByRole("article", {
        name: `${title} block`,
        exact: true,
      });
    await page
      .getByLabel("Stage note", { exact: true })
      .fill("Reasoning draft stays independent.");
    await expect(
      notebook(page).getByRole("article", {
        name: "Reasoning block",
        exact: true,
      }),
    ).toContainText("Unsaved draft");
    await cell("Observation")
      .getByRole("textbox", { name: "Annotation text", exact: true })
      .fill("<script>globalThis.noteExecuted=true</script>");
    await page.keyboard.press("ControlOrMeta+s");
    await expect(
      cell("Observation").getByRole("status", {
        name: "Note save status",
        exact: true,
      }),
    ).toHaveText("Saved");
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Reasoning draft stays independent.",
    );
    await expect(
      notebook(page).getByRole("article", {
        name: "Reasoning block",
        exact: true,
      }),
    ).toContainText("Unsaved draft");
    await expect(
      cell("Alternative").getByRole("textbox", {
        name: "Annotation text",
        exact: true,
      }),
    ).toHaveValue("Inspect the pixels first.");
    assert.equal(
      await page.evaluate(() => Object.hasOwn(globalThis, "noteExecuted")),
      false,
    );
    await page.keyboard.press("ControlOrMeta+Enter");
    assert.equal((await current(page)).exploration.trails.length, 0);
    const concurrent = (await current(page)).exploration;
    const alternative = concurrent.stage.blocks.find(
      (block) => block.kind === "note" && block.note.title === "Alternative",
    );
    assert.ok(alternative?.kind === "note");
    const alternativeText = cell("Alternative").getByRole("textbox", {
      name: "Annotation text",
      exact: true,
    });
    await alternativeText.fill("My independent draft");
    await page.evaluate(
      ({ exploration, note }) =>
        window.play.workspace({
          kind: "save_note",
          context: {
            workspace_id: exploration.workspaceId,
            exploration_id: exploration.id,
            stage_id: exploration.stage.id,
          },
          key: crypto.randomUUID(),
          note_id: note.id,
          expected_revision: note.revision,
          text: "A concurrently saved observation",
        }),
      { exploration: concurrent, note: alternative.note },
    );
    await alternativeText.focus();
    await page.keyboard.press("ControlOrMeta+s");
    await expect(page.getByRole("alert")).toContainText(
      "The saved note changed. Your draft was kept.",
    );
    await expect(alternativeText).toHaveValue("My independent draft");
    await cell("Alternative")
      .getByRole("button", { name: "Discard draft", exact: true })
      .click();
    await expect(alternativeText).toHaveValue(
      "A concurrently saved observation",
    );
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Reasoning draft stays independent.",
    );
    await cell("Observation")
      .getByRole("button", { name: "Duplicate Observation", exact: true })
      .focus();
    await page.keyboard.press("Enter");
    await expect(cell("Observation (copy)")).toBeFocused();
    await expect(cell("Observation (copy)")).toContainText(
      "Copied from Observation · r2",
    );
    const before = await current(page);
    const blockId = await cell("Observation").getAttribute("data-block-id");
    const original = before.exploration.stage.blocks.find(
      (b) => b.id === blockId,
    );
    assert.ok(original?.kind === "note");
    await cell("Observation")
      .getByRole("textbox", { name: "Annotation text", exact: true })
      .fill("Unsaved variation");
    await expect(
      cell("Observation").getByRole("button", {
        name: "Archive Observation",
        exact: true,
      }),
    ).toBeDisabled();
    await cell("Observation")
      .getByRole("button", { name: "Discard draft", exact: true })
      .click();
    await cell("Observation")
      .getByRole("button", { name: "Archive Observation", exact: true })
      .click();
    await notebook(page)
      .getByRole("button", { name: "Archived blocks (1)", exact: true })
      .click();
    const archive = page.getByRole("region", {
      name: "Archived blocks",
      exact: true,
    });
    await expect(archive).toContainText(
      "<script>globalThis.noteExecuted=true</script>",
    );
    await expect(
      archive.getByRole("button", { name: /Open evidence/ }),
    ).toHaveCount(0);
    await archive
      .getByRole("button", { name: "Restore Observation", exact: true })
      .focus();
    await page.keyboard.press("Enter");
    await expect(cell("Observation")).toBeFocused();
    assert.deepEqual(
      (await current(page)).exploration.stage.blocks.find(
        (b) => b.id === original.id,
      ),
      original,
    );
    await cell("Alternative")
      .getByRole("button", { name: "Move Alternative down", exact: true })
      .click();
    await expect(cell("Alternative")).toBeFocused();
    await saveNote(page, "Reasoning draft stays independent.");
    await create(page, "Exploration", "Separate notes");
    await expect(notebook(page)).not.toContainText("Observation");
    await explorer(page)
      .getByRole("button", { name: "Observations", exact: true })
      .click();
    await expect(cell("Observation")).toBeVisible();
    const saved = await current(page);
    await application.evaluate(({ app }) => app.quit());
    await application.close();
    application = await launch();
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(cell("Observation")).toBeVisible();
    assert.deepEqual(await current(page), saved);
    await application.evaluate(({ app }) => app.quit());
  } finally {
    await application
      .evaluate(({ dialog }) => {
        dialog.showMessageBox = async () => ({
          response: 1,
          checkboxChecked: false,
        });
      })
      .catch(() => {});
    await application.close().catch(() => {});
    await rm(profile, { recursive: true, force: true });
  }
});

test("saved output Blocks preserve the original Run and can feed a new text operation", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-result-blocks-"));
  const launch = () =>
    electron.launch({ args: [main, `--user-data-dir=${profile}`] });
  let application = await launch();
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Selected evidence");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Result study");
    await notebook(page)
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    let dialog = page.getByRole("dialog", {
      name: "Add text block",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Clue");
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Gur xrl.");
    await dialog.getByRole("button", { name: "Add text", exact: true }).click();
    const clue = () =>
      notebook(page).getByRole("article", { name: "Clue block", exact: true });
    await clue().getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(clue()).toContainText("output: The key.");
    await page
      .getByLabel("Stage note", { exact: true })
      .fill("Keep this selected result as evidence.");
    const stale = await current(page);
    await page.evaluate(async ({ workspace, exploration }) => {
      await window.play.workspace({
        kind: "reorder_blocks",
        context: {
          workspace_id: workspace.id,
          exploration_id: exploration.id,
          stage_id: exploration.stage.id,
        },
        key: crypto.randomUUID(),
        expected_revision: exploration.stage.revision,
        block_ids: exploration.stage.blocks.map((block) => block.id).reverse(),
      });
    }, stale);
    await clue()
      .getByRole("button", { name: "Save output as block", exact: true })
      .click();
    await expect(page.getByRole("alert")).toContainText(
      "Notebook content changed",
    );
    await expect(
      notebook(page).getByRole("article", {
        name: "ROT13 · output · Run 1 block",
        exact: true,
      }),
    ).toHaveCount(0);
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Keep this selected result as evidence.",
    );
    await expect(
      clue().getByRole("button", { name: "Save output as block", exact: true }),
    ).toBeEnabled();
    await clue()
      .getByRole("button", { name: "Save output as block", exact: true })
      .focus();
    await page.keyboard.press("Enter");
    const result = () =>
      notebook(page).getByRole("article", {
        name: "ROT13 · output · Run 1 block",
        exact: true,
      });
    await expect(result()).toBeFocused();
    await expect(result()).toContainText("Saved from ROT13");
    await expect(result()).toContainText("The key.");
    await expect(
      result().getByRole("button", { name: "Edit text", exact: true }),
    ).toHaveCount(0);
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Keep this selected result as evidence.",
    );
    const saved = await current(page);
    const root = saved.exploration.stage.blocks.find(
      (b) => b.kind === "asset" && b.asset.title === "ROT13 · output · Run 1",
    );
    assert.ok(root?.kind === "asset");
    await result()
      .getByRole("button", { name: "Run cell", exact: true })
      .click();
    await expect(result()).toContainText("output: Gur xrl.");
    await clue()
      .getByRole("button", { name: "Edit text", exact: true })
      .click();
    dialog = page.getByRole("dialog", { name: "Edit text", exact: true });
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Gur frpbaq.");
    await dialog
      .getByRole("button", { name: "Save changes", exact: true })
      .click();
    await clue().getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(clue()).toContainText("output: The second.");
    await result()
      .getByRole("button", { name: "Open source Run 1", exact: true })
      .click();
    const canvas = () =>
      page.getByRole("region", { name: "Trail canvas", exact: true });
    await expect(canvas()).toContainText("Earlier input · r1");
    await expect(canvas()).toContainText("output: The key.");
    await expect(
      canvas().getByRole("button", {
        name: "Saved output as block",
        exact: true,
      }),
    ).toBeDisabled();
    await page.getByRole("tab", { name: "Stage 1", exact: true }).click();
    await clue()
      .getByRole("button", { name: "Archive Clue", exact: true })
      .click();
    await result()
      .getByRole("button", { name: "Open source Run 1", exact: true })
      .click();
    await expect(canvas()).toContainText("Archived block");
    await expect(canvas()).toContainText("output: The key.");
    await page.getByRole("tab", { name: "Stage 1", exact: true }).click();
    await result()
      .getByRole("button", {
        name: "Archive ROT13 · output · Run 1",
        exact: true,
      })
      .click();
    await notebook(page)
      .getByRole("button", { name: "Archived blocks (2)", exact: true })
      .click();
    const archive = page.getByRole("region", {
      name: "Archived blocks",
      exact: true,
    });
    await archive
      .getByRole("button", {
        name: "Restore ROT13 · output · Run 1",
        exact: true,
      })
      .click();
    await expect(result()).toBeFocused();
    assert.deepEqual(
      (await current(page)).exploration.stage.blocks.find(
        (b) => b.id === root.id,
      ),
      root,
    );
    await saveNote(page, "Keep this selected result as evidence.");
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, fixture);
    await notebook(page)
      .getByRole("button", { name: "Add image", exact: true })
      .click();
    dialog = page.getByRole("dialog", {
      name: "Add image with caption",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Caption", exact: true })
      .fill("Grouped with the image.");
    await dialog
      .getByRole("button", { name: "Choose PNG", exact: true })
      .click();
    const file = join(profile, "evidence.bin");
    await writeFile(file, Buffer.from([0, 128, 255]));
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, file);
    await notebook(page)
      .getByRole("button", { name: "Add file", exact: true })
      .click();
    for (let index = 0; index < 2; index++) {
      await result()
        .getByRole("button", {
          name: "Move ROT13 · output · Run 1 down",
          exact: true,
        })
        .click();
      await expect(result()).toBeFocused();
    }
    const mixed = (await current(page)).exploration.stage.blocks.filter(
      (block) => block.kind === "asset",
    );
    assert.equal(mixed[0]?.asset.content.kind, "image");
    assert.equal(mixed[1]?.asset.content.kind, "file");
    assert.equal(mixed[2]?.asset.id, root.asset.id);
    await create(page, "Exploration", "Separate results");
    await expect(notebook(page)).not.toContainText("Saved from ROT13");
    await explorer(page)
      .getByRole("button", { name: "Result study", exact: true })
      .click();
    await expect(result()).toContainText("output: Gur xrl.");
    const before = await current(page);
    await application.evaluate(({ app }) => app.quit());
    await application.close();
    application = await launch();
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(result()).toContainText("Saved from ROT13");
    await expect(result()).toContainText("output: Gur xrl.");
    assert.deepEqual(await current(page), before);
    assert.deepEqual(await readFile(file), Buffer.from([0, 128, 255]));
    await application.evaluate(({ app }) => app.quit());
  } finally {
    await application
      .evaluate(({ dialog }) => {
        dialog.showMessageBox = async () => ({
          response: 1,
          checkboxChecked: false,
        });
      })
      .catch(() => {});
    await application.close().catch(() => {});
    await rm(profile, { recursive: true, force: true });
  }
});

test("File Blocks import inert content and preserve scoped copies, order and archive after restart", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-file-blocks-"));
  const path = join(profile, "clue.html");
  const text =
    "<h1>Injected heading</h1><script>globalThis.fileExecuted=true</script>\nGur xrl.";
  await writeFile(path, text);
  const launch = () =>
    electron.launch({ args: [main, `--user-data-dir=${profile}`] });
  let application = await launch();
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("File evidence");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Inert clues");
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, path);
    const add = () =>
      notebook(page).getByRole("button", { name: "Add file", exact: true });
    await add().focus();
    await page.keyboard.press("Enter");
    const cell = () =>
      notebook(page).getByRole("article", {
        name: "clue.html block",
        exact: true,
      });
    await expect(cell()).toContainText(text);
    await expect(
      cell().getByRole("button", { name: "Edit text", exact: true }),
    ).toHaveCount(0);
    await expect(
      cell().getByRole("button", { name: "Run cell", exact: true }),
    ).toHaveCount(0);
    const initial = await current(page);
    const original = initial.exploration.stage.blocks[0];
    assert.ok(original?.kind === "asset");
    await cell()
      .getByRole("button", { name: "Explore clue.html", exact: true })
      .click();
    const canvas = () =>
      page.getByRole("region", { name: "Trail canvas", exact: true });
    await expect(canvas()).toContainText("File inspection");
    await expect(canvas().locator("pre")).toHaveText(text);
    await expect(
      canvas().getByRole("heading", { name: "Injected heading", exact: true }),
    ).toHaveCount(0);
    assert.equal(
      await page.evaluate(() => Object.hasOwn(globalThis, "fileExecuted")),
      false,
    );
    await page.keyboard.press("ControlOrMeta+Enter");
    await expect(
      canvas().getByRole("button", { name: "Run cell", exact: true }),
    ).toHaveCount(0);
    await page.getByRole("tab", { name: "Stage 1", exact: true }).click();
    await cell()
      .getByRole("button", { name: "Duplicate clue.html", exact: true })
      .click();
    const copy = notebook(page).getByRole("article", {
      name: "clue.html (copy) block",
      exact: true,
    });
    await expect(copy).toBeFocused();
    await expect(copy).toContainText("Copied from clue.html");
    await cell()
      .getByRole("button", { name: "Move clue.html down", exact: true })
      .click();
    await expect(cell()).toBeFocused();
    await cell()
      .getByRole("button", { name: "Archive clue.html", exact: true })
      .click();
    await expect(cell()).toHaveCount(0);
    await notebook(page)
      .getByRole("button", { name: "Archived blocks (1)", exact: true })
      .click();
    const archive = page.getByRole("region", {
      name: "Archived blocks",
      exact: true,
    });
    await archive
      .getByRole("button", { name: "Open evidence clue.html", exact: true })
      .click();
    await expect(canvas()).toContainText("Archived block");
    await expect(canvas().locator("pre")).toHaveText(text);
    await page.getByRole("tab", { name: "Stage 1", exact: true }).click();
    await notebook(page)
      .getByRole("button", { name: "Archived blocks (1)", exact: true })
      .click();
    await archive
      .getByRole("button", { name: "Restore clue.html", exact: true })
      .click();
    await expect(cell()).toBeFocused();
    const restored = await current(page);
    assert.deepEqual(restored.exploration.stage.blocks[1], original);
    const binaryPath = join(profile, "data.bin");
    await writeFile(binaryPath, Buffer.from([0, 255, 1, 128]));
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, binaryPath);
    await add().click();
    const binaryCell = notebook(page).getByRole("article", {
      name: "data.bin block",
      exact: true,
    });
    await expect(binaryCell).toContainText("00ff0180");
    const beforeCancel = await current(page);
    await application.evaluate(({ dialog }) => {
      dialog.showOpenDialog = async () => ({ canceled: true, filePaths: [] });
    });
    await add().click();
    await expect(add()).toBeEnabled();
    assert.deepEqual(await current(page), beforeCancel);
    await create(page, "Exploration", "Separate files");
    await expect(notebook(page)).not.toContainText("clue.html");
    await explorer(page)
      .getByRole("button", { name: "Inert clues", exact: true })
      .click();
    await expect(cell()).toBeVisible();
    const saved = await current(page);
    await application.evaluate(({ app }) => app.quit());
    await application.close();
    application = await launch();
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(cell()).toBeVisible();
    assert.deepEqual(await current(page), saved);
    assert.deepEqual(await readFile(path), Buffer.from(text));
    await application.evaluate(({ app }) => app.quit());
  } finally {
    await application.close().catch(() => {});
    await rm(profile, { recursive: true, force: true });
  }
});

test("archived Blocks keep evidence and restore their original Assets after restart", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-archive-"));
  const imported = await readFile(fixture);
  const launch = () =>
    electron.launch({ args: [main, `--user-data-dir=${profile}`] });
  let application = await launch();
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Preserved clues");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Archive study");
    await notebook(page)
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    let dialog = page.getByRole("dialog", {
      name: "Add text block",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Clue");
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Gur xrl.");
    await dialog.getByRole("button", { name: "Add text", exact: true }).click();
    const cell = () =>
      notebook(page).getByRole("article", { name: "Clue block", exact: true });
    if (process.platform === "darwin") {
      await cell()
        .getByRole("button", { name: "Run cell", exact: true })
        .click();
      await expect(cell()).toContainText("output: The key.");
    }
    const before = await current(page);
    const source = before.exploration.stage.blocks[0];
    assert.ok(source?.kind === "asset");
    const context = {
      workspace_id: before.workspace.id,
      exploration_id: before.exploration.id,
      stage_id: before.exploration.stage.id,
    };
    const trail = before.exploration.trails.find(
      (t) => t.source.id === source.asset.id,
    );
    assert.ok(trail);
    const history = () =>
      page.evaluate(
        async ({ context, id }) =>
          window.play.workspace({
            kind: "list_runs",
            context,
            trail_id: id,
            before: null,
          }),
        { context, id: trail.id },
      );
    const saved = (await history()).runs;
    await page
      .getByLabel("Stage note", { exact: true })
      .fill("Evidence stays available while this clue is archived.");
    await cell()
      .getByRole("button", { name: "Archive Clue", exact: true })
      .click();
    await expect(cell()).toHaveCount(0);
    await expect(
      notebook(page).getByRole("article", {
        name: "Reasoning block",
        exact: true,
      }),
    ).toBeFocused();
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Evidence stays available while this clue is archived.",
    );
    const openArchive = async () => {
      await notebook(page)
        .getByRole("button", { name: "Archived blocks (1)", exact: true })
        .click();
    };
    await openArchive();
    let archive = page.getByRole("region", {
      name: "Archived blocks",
      exact: true,
    });
    await expect(archive).toContainText("Gur xrl.");
    await archive
      .getByRole("button", { name: "Open evidence Clue", exact: true })
      .click();
    const canvas = page.getByRole("region", {
      name: "Trail canvas",
      exact: true,
    });
    await expect(canvas).toContainText("Archived block");
    await expect(
      canvas.getByRole("button", { name: "Run cell", exact: true }),
    ).toBeDisabled();
    await page.keyboard.press("ControlOrMeta+Enter");
    assert.deepEqual((await history()).runs, saved);
    if (process.platform === "darwin")
      await expect(canvas).toContainText("output: The key.");
    await page.getByRole("tab", { name: "Stage 1", exact: true }).click();
    await openArchive();
    archive = page.getByRole("region", {
      name: "Archived blocks",
      exact: true,
    });
    const restore = archive.getByRole("button", {
      name: "Restore Clue",
      exact: true,
    });
    await expect(restore).toBeEnabled();
    await restore.focus();
    await page.keyboard.press("Enter");
    await expect(cell()).toBeFocused();
    assert.deepEqual((await current(page)).exploration.stage.blocks[0], source);
    if (process.platform === "darwin") {
      await cell()
        .getByRole("button", { name: "Run cell", exact: true })
        .click();
      await expect.poll(async () => (await history()).runs.length).toBe(2);
    }
    await page.evaluate(
      async ({ context, id, asset }) => {
        await window.play.workspace({
          kind: "revise_text",
          context,
          key: crypto.randomUUID(),
          block_id: id,
          asset_id: asset,
          expected_revision: 1,
          text: "Frpbaq.",
        });
      },
      { context, id: source.id, asset: source.asset.id },
    );
    await cell()
      .getByRole("button", { name: "Archive Clue", exact: true })
      .click();
    await expect(page.getByRole("alert")).toContainText(
      "Notebook or saved block changed",
    );
    await expect(cell()).toContainText("Frpbaq.");
    assert.equal(
      (await current(page)).exploration.stage.archivedBlocks.length,
      0,
    );
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, fixture);
    await notebook(page)
      .getByRole("button", { name: "Add image", exact: true })
      .click();
    dialog = page.getByRole("dialog", {
      name: "Add image with caption",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Caption", exact: true })
      .fill("Gur cvpgher.");
    await dialog
      .getByRole("button", { name: "Choose PNG", exact: true })
      .click();
    const image = () =>
      notebook(page).getByRole("article", {
        name: "sample.png block",
        exact: true,
      });
    const imageBefore = (await current(page)).exploration.stage.blocks.find(
      (b) => b.kind === "asset" && b.asset.content.kind === "image",
    );
    assert.ok(imageBefore);
    await image()
      .getByRole("button", { name: "Archive sample.png", exact: true })
      .click();
    await expect(image()).toHaveCount(0);
    await openArchive();
    archive = page.getByRole("region", {
      name: "Archived blocks",
      exact: true,
    });
    await expect(archive.getByRole("figure")).toContainText("Gur cvpgher.");
    await expect(
      archive.getByRole("img", { name: "sample.png", exact: true }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: "Save reasoning", exact: true })
      .click();
    await expect(
      page.getByRole("status", { name: "Note save status" }),
    ).toHaveText("Saved");
    await capture(application, "native-archived-blocks");
    const archivedView = (await current(page)).exploration;
    await application.close();
    application = await launch();
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(notebook(page)).toBeVisible();
    assert.deepEqual((await current(page)).exploration, archivedView);
    await openArchive();
    archive = page.getByRole("region", {
      name: "Archived blocks",
      exact: true,
    });
    await archive
      .getByRole("button", { name: "Restore sample.png", exact: true })
      .click();
    await expect(image()).toBeFocused();
    assert.deepEqual(
      (await current(page)).exploration.stage.blocks.find(
        (b) => b.id === imageBefore.id,
      ),
      imageBefore,
    );
    assert.deepEqual(await readFile(fixture), imported);
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

test("duplicated Blocks preserve independent content and source lineage after restart", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-duplicate-"));
  const importedSource = await readFile(fixture);
  const launch = () =>
    electron.launch({ args: [main, `--user-data-dir=${profile}`] });
  let application = await launch();
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Copied clues");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Clue variants");
    await notebook(page)
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    let dialog = page.getByRole("dialog", {
      name: "Add text block",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Clue");
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Svefg.");
    await dialog.getByRole("button", { name: "Add text", exact: true }).click();
    const cell = (title: string) =>
      notebook(page).getByRole("article", {
        name: `${title} block`,
        exact: true,
      });
    if (process.platform === "darwin") {
      await cell("Clue")
        .getByRole("button", { name: "Run cell", exact: true })
        .click();
      await expect(cell("Clue")).toContainText("output: First.");
    }
    await page
      .getByLabel("Stage note", { exact: true })
      .fill("Try a second clue without changing the first.");
    await cell("Clue")
      .getByRole("button", { name: "Duplicate Clue", exact: true })
      .click();
    await expect(cell("Clue (copy)")).toBeFocused();
    await expect(cell("Clue (copy)")).toContainText("Copied from Clue · r1");
    await expect(
      cell("Clue (copy)").getByRole("region", { name: "Cell result" }),
    ).toHaveCount(0);
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Try a second clue without changing the first.",
    );
    await expect(
      cell("Reasoning").getByRole("button", { name: /Duplicate/ }),
    ).toHaveCount(0);
    const edit = async (title: string, caption: boolean, text: string) => {
      await cell(title)
        .getByRole("button", {
          name: caption ? "Edit caption" : "Edit text",
          exact: true,
        })
        .click();
      const editor = page.getByRole("dialog", {
        name: caption ? "Edit caption" : "Edit text",
        exact: true,
      });
      await editor
        .getByRole("textbox", {
          name: caption ? "Caption" : "Text",
          exact: true,
        })
        .fill(text);
      await editor
        .getByRole("button", { name: "Save changes", exact: true })
        .click();
      await expect(editor).toBeHidden();
    };
    await edit("Clue (copy)", false, "Frpbaq.");
    await expect(cell("Clue")).toContainText("Svefg.");
    if (process.platform === "darwin") {
      await cell("Clue (copy)").focus();
      await page.keyboard.press("ControlOrMeta+Enter");
      await expect(cell("Clue (copy)")).toContainText("output: Second.");
      await expect(cell("Clue")).toContainText("output: First.");
    }
    const view = await current(page);
    const source = view.exploration.stage.blocks[0];
    assert.ok(source?.kind === "asset");
    const context = {
      workspace_id: view.workspace.id,
      exploration_id: view.exploration.id,
      stage_id: view.exploration.stage.id,
    };
    await page.evaluate(
      async ({ context, id, asset }) => {
        await window.play.workspace({
          kind: "revise_text",
          context,
          key: crypto.randomUUID(),
          block_id: id,
          asset_id: asset,
          expected_revision: 1,
          text: "Gur svefg.",
        });
      },
      { context, id: source.id, asset: source.asset.id },
    );
    await cell("Clue")
      .getByRole("button", { name: "Duplicate Clue", exact: true })
      .click();
    await expect(page.getByRole("alert")).toContainText("Saved block changed");
    assert.equal((await current(page)).exploration.stage.blocks.length, 3);
    await cell("Clue")
      .getByRole("button", { name: "Duplicate Clue", exact: true })
      .focus();
    await page.keyboard.press("Enter");
    await expect(cell("Clue (copy 2)")).toBeFocused();
    await expect(cell("Clue (copy 2)")).toContainText("Copied from Clue · r2");
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, fixture);
    await notebook(page)
      .getByRole("button", { name: "Add image", exact: true })
      .click();
    dialog = page.getByRole("dialog", {
      name: "Add image with caption",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Caption", exact: true })
      .fill("Gur xrl.");
    await dialog
      .getByRole("button", { name: "Choose PNG", exact: true })
      .click();
    await cell("sample.png")
      .getByRole("button", { name: "Duplicate sample.png", exact: true })
      .click();
    await expect(cell("sample.png (copy)")).toBeFocused();
    await expect(
      cell("sample.png (copy)").getByRole("img", {
        name: "sample.png (copy)",
        exact: true,
      }),
    ).toBeVisible();
    await expect(cell("sample.png (copy)").getByRole("figure")).toContainText(
      "Gur xrl.",
    );
    await edit("sample.png (copy)", true, "Gur frpbaq.");
    await expect(cell("sample.png").getByRole("figure")).toContainText(
      "Gur xrl.",
    );
    if (process.platform === "darwin") {
      await cell("sample.png (copy)")
        .getByRole("button", { name: "Run cell", exact: true })
        .click();
      await expect(cell("sample.png (copy)")).toContainText(
        "output: The second.",
      );
    }
    await cell("sample.png (copy)")
      .getByRole("button", {
        name: "Explore Image caption (copy)",
        exact: true,
      })
      .click();
    await expect(
      page.getByRole("region", { name: "Trail canvas", exact: true }),
    ).toContainText("Gur frpbaq.");
    if (process.platform === "darwin")
      await expect(
        page.getByRole("region", { name: "Trail canvas", exact: true }),
      ).toContainText("output: The second.");
    await page.getByRole("tab", { name: "Stage 1", exact: true }).click();
    await page
      .getByRole("button", { name: "Save reasoning", exact: true })
      .click();
    await expect(
      page.getByRole("status", { name: "Note save status" }),
    ).toHaveText("Saved");
    await capture(application, "native-duplicated-blocks");
    for (const number of [3, 4]) {
      await cell("Clue")
        .getByRole("button", { name: "Duplicate Clue", exact: true })
        .click();
      await expect(cell(`Clue (copy ${number})`)).toBeFocused();
    }
    await expect(
      cell("Clue").getByRole("button", { name: "Duplicate Clue", exact: true }),
    ).toBeDisabled();
    const saved = (await current(page)).exploration.stage.blocks;
    assert.equal(saved.length, 8);
    await application.close();
    application = await launch();
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(notebook(page)).toBeVisible();
    assert.deepEqual((await current(page)).exploration.stage.blocks, saved);
    assert.deepEqual(await readFile(fixture), importedSource);
    await expect(cell("Clue (copy)")).toContainText("Copied from Clue · r1");
    if (process.platform === "darwin") {
      await expect(cell("Clue (copy)")).toContainText("output: Second.");
      await expect(cell("sample.png (copy)")).toContainText(
        "output: The second.",
      );
    }
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
    notebook(page)
      .getByRole("article", { name: "Reasoning block", exact: true })
      .getByRole("status", { name: "Note save status" }),
  ).toHaveText("Saved");
}
async function capture(application: ElectronApplication, name: string) {
  await mkdir("playwright-report", { recursive: true });
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
  const denied = await rogue.evaluate(
    async ({ url, context }) => {
      const results = [];
      for (const invoke of [
        () => window.play.catalog(),
        () => window.play.workspace({ kind: "list" }),
        () => window.play.importFile({ context, key: crypto.randomUUID() }),
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
    },
    { url: imageUrl, context },
  );
  assert.deepEqual(
    denied,
    [true, true, true, true],
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
    await notebook(page)
      .getByText("Keep result in private history", { exact: true })
      .click();
    await expect(
      notebook(page).getByRole("checkbox", {
        name: "Keep result in private history",
      }),
    ).not.toBeChecked();
    await notebook(page)
      .getByRole("button", { name: "Run cell", exact: true })
      .click();
    await expect(
      notebook(page).getByRole("region", { name: "Cell result" }),
    ).toContainText("output was not retained");
    const wrongOwner = await page.evaluate(
      async ({ context, trailId, pipelineId }) => {
        try {
          await window.play.startRun({
            context,
            key: crypto.randomUUID(),
            trail_id: trailId,
            pipeline_id: pipelineId,
            pipeline_revision: 1,
            source_asset_id: crypto.randomUUID(),
            source_revision: 1,
            retain_output: true,
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
        trailId:
          first.exploration.trails.find(
            (trail) => trail.source.content.kind === "text",
          )?.id ?? "",
        pipelineId: first.workspace.pipelines[0]?.id ?? "",
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
      restored.exploration.stage.blocks.find((block) => block.kind === "asset")
        ?.asset.id,
      first.exploration.stage.blocks.find((block) => block.kind === "asset")
        ?.asset.id,
    );
    await expect(
      notebook(page).getByRole("img", { name: "sample.png", exact: true }),
    ).toBeVisible();
    await expect(
      notebook(page).getByRole("region", { name: "Cell result" }),
    ).toContainText("The key is in the pixels.");
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
    await expect(
      notebook(page).getByRole("region", { name: "Cell result" }),
    ).toContainText("output was not retained");
    await explorer(page)
      .getByRole("button", { name: "Spiral signal", exact: true })
      .click();
    const restoredResult = notebook(page).getByRole("region", {
      name: "Cell result",
    });
    await restoredResult
      .getByText("Saved Run & input evidence", { exact: false })
      .click();
    await restoredResult
      .getByRole("button", { name: "Open Run 1", exact: true })
      .click();
    await expect(restoredResult).toContainText("Out [1]");
    await expect(restoredResult).toContainText("The key is in the pixels.");
    await expect(restoredResult.locator("details")).toHaveAttribute("open", "");
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

test("native text and caption revisions preserve historical Runs and drafts across restart", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-edits-"));
  const sourceBefore = await readFile(fixture);
  const launch = () =>
    electron.launch({ args: [main, `--user-data-dir=${profile}`] });
  let application = await launch();
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Revision studies");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Changing clues");
    await notebook(page)
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    let dialog = page.getByRole("dialog", {
      name: "Add text block",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Clue");
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Gur xrl.");
    await dialog.getByRole("button", { name: "Add text", exact: true }).click();
    let cell = notebook(page).getByRole("article", {
      name: "Clue block",
      exact: true,
    });
    if (process.platform === "darwin") {
      await cell.getByRole("button", { name: "Run cell", exact: true }).click();
      await expect(
        cell.getByRole("region", { name: "Cell result" }),
      ).toContainText("The key.");
    }
    const original = await current(page);
    const block = original.exploration.stage.blocks.find(
      (block) => block.kind === "asset",
    );
    assert.ok(block);
    await page
      .getByLabel("Stage note", { exact: true })
      .fill("Reasoning is still a separate draft.");
    await cell.getByRole("button", { name: "Edit text", exact: true }).click();
    dialog = page.getByRole("dialog", { name: "Edit text", exact: true });
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Frpbaq pyhr.");
    const runCount = () =>
      page.evaluate(async (exploration) => {
        const trail = exploration.trails[0];
        if (!trail) throw new Error("trail_missing");
        const page = await window.play.workspace({
          kind: "list_runs",
          context: {
            workspace_id: exploration.workspaceId,
            exploration_id: exploration.id,
            stage_id: exploration.stage.id,
          },
          trail_id: trail.id,
          before: null,
        });
        return page.runs.length;
      }, original.exploration);
    const beforeShortcut = await runCount();
    await page.keyboard.press("ControlOrMeta+Enter");
    await expect(dialog).toBeVisible();
    assert.equal(
      await runCount(),
      beforeShortcut,
      "an editor shortcut must not create a Run",
    );
    await page.keyboard.press("ControlOrMeta+s");
    await expect(dialog).toBeHidden();
    await expect(cell).toContainText("Frpbaq pyhr.");
    await expect(cell).toContainText("Asset · r2");
    await expect(
      page.getByRole("status", { name: "Note save status" }),
    ).toHaveText("Unsaved changes");
    const withNoteDraft = await current(page);
    assert.equal(reasoningOf(withNoteDraft.exploration.stage).revision, 1);
    await page
      .getByRole("button", { name: "Save reasoning", exact: true })
      .click();
    await expect(
      page.getByRole("status", { name: "Note save status" }),
    ).toHaveText("Saved");
    if (process.platform === "darwin") {
      const result = cell.getByRole("region", { name: "Cell result" });
      await expect(result).toContainText("Earlier input · r1");
      await expect(result).toContainText("The key.");
      await cell.getByRole("button", { name: "Run cell", exact: true }).click();
      await expect(result).toContainText("Second clue.");
      await expect(result).not.toContainText("Earlier input");
    }
    const revised = await current(page);
    assert.equal(revised.exploration.stage.blocks[0]?.id, block.id);
    assert.equal(
      revised.exploration.stage.blocks.find((block) => block.kind === "asset")
        ?.asset.id,
      block.asset.id,
    );
    await cell.getByRole("button", { name: "Edit text", exact: true }).click();
    dialog = page.getByRole("dialog", { name: "Edit text", exact: true });
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Draft stays here.");
    await page.evaluate(
      async ({ block, context }) => {
        await window.play.workspace({
          kind: "revise_text",
          context,
          key: crypto.randomUUID(),
          block_id: block.id,
          asset_id: block.asset.id,
          expected_revision: 2,
          text: "Another saved revision.",
        });
      },
      {
        block,
        context: {
          workspace_id: original.workspace.id,
          exploration_id: original.exploration.id,
          stage_id: original.exploration.stage.id,
        },
      },
    );
    await dialog
      .getByRole("button", { name: "Save changes", exact: true })
      .click();
    await expect(dialog.getByRole("alert")).toContainText("draft was kept");
    await expect(
      dialog.getByRole("textbox", { name: "Text", exact: true }),
    ).toHaveValue("Draft stays here.");
    await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(cell).toContainText("Another saved revision.");
    await cell.getByRole("button", { name: "Edit text", exact: true }).click();
    dialog = page.getByRole("dialog", { name: "Edit text", exact: true });
    await expect(
      dialog.getByRole("textbox", { name: "Text", exact: true }),
    ).toHaveValue("Another saved revision.");
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Unsaved source draft.");
    if (process.platform === "darwin") {
      await expect
        .poll(() =>
          application.evaluate(({ BrowserWindow }) =>
            BrowserWindow.getAllWindows().some((window) =>
              window.isDocumentEdited(),
            ),
          ),
        )
        .toBe(true);
      const prompt = await application.evaluate(
        ({ app, dialog }) =>
          new Promise<string>((resolve) => {
            dialog.showMessageBox = async (...args: unknown[]) => {
              const options = args.find(
                (value) =>
                  value && typeof value === "object" && "message" in value,
              );
              resolve(
                options &&
                  typeof options === "object" &&
                  "message" in options &&
                  typeof options.message === "string"
                  ? options.message
                  : "missing_message",
              );
              return { response: 0, checkboxChecked: false };
            };
            app.quit();
          }),
      );
      assert.equal(prompt, "Changes have not been saved.");
      await expect(dialog).toBeVisible();
    }
    await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, fixture);
    await notebook(page)
      .getByRole("button", { name: "Add image", exact: true })
      .click();
    dialog = page.getByRole("dialog", {
      name: "Add image with caption",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Caption", exact: true })
      .fill("Gur pncgvba.");
    await dialog
      .getByRole("button", { name: "Choose PNG", exact: true })
      .click();
    const imageCell = notebook(page).getByRole("article", {
      name: "sample.png block",
      exact: true,
    });
    if (process.platform === "darwin") {
      await imageCell
        .getByRole("button", { name: "Run cell", exact: true })
        .click();
      await expect(
        imageCell.getByRole("region", { name: "Cell result" }),
      ).toContainText("The caption.");
    }
    await imageCell
      .getByRole("button", { name: "Edit caption", exact: true })
      .click();
    dialog = page.getByRole("dialog", { name: "Edit caption", exact: true });
    await dialog
      .getByRole("textbox", { name: "Caption", exact: true })
      .fill("Frpbaq pncgvba.");
    await capture(application, "native-caption-editor");
    await page.keyboard.press("ControlOrMeta+s");
    await expect(dialog).toBeHidden();
    await expect(imageCell.getByRole("figure")).toContainText(
      "Frpbaq pncgvba.",
    );
    await expect(imageCell.getByRole("figure")).toContainText("Caption · r2");
    await expect(
      imageCell.getByRole("img", { name: "sample.png", exact: true }),
    ).toBeVisible();
    if (process.platform === "darwin") {
      await expect(
        imageCell.getByRole("region", { name: "Cell result" }),
      ).toContainText("Earlier input · r1");
      await capture(application, "native-edited-notebook");
      await imageCell
        .getByRole("button", { name: "Run cell", exact: true })
        .click();
      await expect(
        imageCell.getByRole("region", { name: "Cell result" }),
      ).toContainText("Second caption.");
    }
    await application.close();
    application = await launch();
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    cell = notebook(page).getByRole("article", {
      name: "Clue block",
      exact: true,
    });
    await expect(cell).toContainText("Another saved revision.");
    const restored = await current(page);
    assert.equal(
      restored.exploration.stage.blocks.find((block) => block.kind === "asset")
        ?.asset.revision,
      3,
    );
    await expect(
      notebook(page).getByRole("figure").filter({ hasText: "Frpbaq pncgvba." }),
    ).toBeVisible();
    if (process.platform === "darwin") {
      await expect(
        notebook(page).getByRole("article", {
          name: "sample.png block",
          exact: true,
        }),
      ).toContainText("Second caption.");
      await expect(
        cell.getByRole("region", { name: "Cell result" }),
      ).toContainText("Earlier input · r2");
    }
    assert.deepEqual(await readFile(fixture), sourceBefore);
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

test("ordered notebook Blocks preserve drafts, evidence and keyboard context after restart", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-block-order-"));
  const sourceBefore = await readFile(fixture);
  const launch = () =>
    electron.launch({ args: [main, `--user-data-dir=${profile}`] });
  let application = await launch();
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Ordered studies");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Clue order");
    for (const [title, text] of [
      ["First clue", "Svefg."],
      ["Second clue", "Frpbaq."],
    ]) {
      await notebook(page)
        .getByRole("button", { name: "Add text", exact: true })
        .click();
      const dialog = page.getByRole("dialog", {
        name: "Add text block",
        exact: true,
      });
      await dialog
        .getByRole("textbox", { name: "Title", exact: true })
        .fill(title ?? "");
      await dialog
        .getByRole("textbox", { name: "Text", exact: true })
        .fill(text ?? "");
      await dialog
        .getByRole("button", { name: "Add text", exact: true })
        .click();
    }
    const firstCell = () =>
      notebook(page).getByRole("article", {
        name: "First clue block",
        exact: true,
      });
    await expect(
      firstCell().getByRole("button", {
        name: "Move First clue up",
        exact: true,
      }),
    ).toBeDisabled();
    await expect(
      notebook(page).getByRole("button", {
        name: "Move Reasoning down",
        exact: true,
      }),
    ).toBeDisabled();
    if (process.platform === "darwin") {
      await firstCell()
        .getByRole("button", { name: "Run cell", exact: true })
        .click();
      await expect(firstCell()).toContainText("output: First.");
    }
    const original = await current(page);
    const before = original.exploration.stage.blocks;
    const first = before.find(
      (block) => block.kind === "asset" && block.asset.title === "First clue",
    );
    const second = before.find(
      (block) => block.kind === "asset" && block.asset.title === "Second clue",
    );
    const reasoning = before.find((block) => block.kind === "note");
    assert.ok(
      first?.kind === "asset" &&
        second?.kind === "asset" &&
        reasoning?.kind === "note",
    );
    const originalContext = {
      workspace_id: original.workspace.id,
      exploration_id: original.exploration.id,
      stage_id: original.exploration.stage.id,
    };
    const firstTrail = original.exploration.trails.find(
      (trail) => trail.source.id === first.asset.id,
    );
    assert.ok(firstTrail);
    const savedRun = await page.evaluate(
      async ({ context, trailId }) => {
        const history = await window.play.workspace({
          kind: "list_runs",
          context,
          trail_id: trailId,
          before: null,
        });
        const id = history.runs[0]?.id;
        return id
          ? window.play.workspace({ kind: "run_detail", context, run_id: id })
          : null;
      },
      { context: originalContext, trailId: firstTrail.id },
    );
    await page
      .getByLabel("Stage note", { exact: true })
      .fill("Reasoning stays with this Block.");
    const secondCell = () =>
      notebook(page).getByRole("article", {
        name: "Second clue block",
        exact: true,
      });
    await secondCell()
      .getByRole("button", { name: "Move Second clue up", exact: true })
      .click();
    await expect
      .poll(async () =>
        (await current(page)).exploration.stage.blocks.map((block) => block.id),
      )
      .toEqual([second.id, first.id, reasoning.id]);
    await expect(secondCell()).toBeFocused();
    const noteCell = () =>
      notebook(page).getByRole("article", {
        name: "Reasoning block",
        exact: true,
      });
    await expect(
      noteCell().getByRole("button", {
        name: "Move Reasoning up",
        exact: true,
      }),
    ).toBeEnabled();
    await noteCell()
      .getByRole("button", { name: "Move Reasoning up", exact: true })
      .focus();
    await page.keyboard.press("Enter");
    await expect
      .poll(async () =>
        (await current(page)).exploration.stage.blocks.map((block) => block.id),
      )
      .toEqual([second.id, reasoning.id, first.id]);
    await expect(noteCell()).toBeFocused();
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Reasoning stays with this Block.",
    );
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
      exact: true,
    });
    await imageDialog
      .getByRole("textbox", { name: "Caption", exact: true })
      .fill("Gur cvpgher.");
    await imageDialog
      .getByRole("button", { name: "Choose PNG", exact: true })
      .click();
    const withImage = await current(page);
    const image = withImage.exploration.stage.blocks.find(
      (block) => block.kind === "asset" && block.asset.content.kind === "image",
    );
    assert.ok(image);
    const imageCell = () =>
      notebook(page).getByRole("article", {
        name: "sample.png block",
        exact: true,
      });
    await imageCell()
      .getByRole("button", { name: "Move sample.png up", exact: true })
      .click();
    await expect
      .poll(async () =>
        (await current(page)).exploration.stage.blocks.map((block) => block.id),
      )
      .toEqual([second.id, reasoning.id, image.id, first.id]);
    await expect(imageCell()).toBeFocused();
    const displayed = await current(page);
    const external = displayed.exploration.stage.blocks.map(
      (block) => block.id,
    );
    [external[0], external[1]] = [external[1] ?? "", external[0] ?? ""];
    await page.evaluate(
      async ({ context, revision, ids }) => {
        await window.play.workspace({
          kind: "reorder_blocks",
          context,
          key: crypto.randomUUID(),
          expected_revision: revision,
          block_ids: ids,
        });
      },
      {
        context: originalContext,
        revision: displayed.exploration.stage.revision,
        ids: external,
      },
    );
    await imageCell()
      .getByRole("button", { name: "Move sample.png up", exact: true })
      .click();
    await expect(page.getByRole("alert")).toContainText(
      "Notebook order changed",
    );
    await expect
      .poll(async () =>
        (await current(page)).exploration.stage.blocks.map((block) => block.id),
      )
      .toEqual(external);
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Reasoning stays with this Block.",
    );
    await imageCell()
      .getByRole("button", { name: "Move sample.png up", exact: true })
      .click();
    const finalIds = [reasoning.id, image.id, second.id, first.id];
    await expect
      .poll(async () =>
        (await current(page)).exploration.stage.blocks.map((block) => block.id),
      )
      .toEqual(finalIds);
    await expect(imageCell()).toBeFocused();
    await expect(imageCell().getByRole("figure")).toContainText("Gur cvpgher.");
    await expect(
      imageCell().getByRole("img", { name: "sample.png", exact: true }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: "Save reasoning", exact: true })
      .click();
    await expect(
      page.getByRole("status", { name: "Note save status" }),
    ).toHaveText("Saved");
    if (process.platform === "darwin") {
      await secondCell().focus();
      await page.keyboard.press("ControlOrMeta+Enter");
      await expect(secondCell()).toContainText("output: Second.");
      const allTrails = (await current(page)).exploration.trails;
      const runCount = () =>
        page.evaluate(
          async ({ context, trails }) => {
            let count = 0;
            for (const trail of trails) {
              const history = await window.play.workspace({
                kind: "list_runs",
                context,
                trail_id: trail.id,
                before: null,
              });
              count += history.runs.length;
            }
            return count;
          },
          { context: originalContext, trails: allTrails },
        );
      const count = await runCount();
      await page.getByLabel("Stage note", { exact: true }).focus();
      await page.keyboard.press("ControlOrMeta+Enter");
      assert.equal(
        await runCount(),
        count,
        "reasoning does not dispatch another Block's Run",
      );
      if (savedRun)
        assert.deepEqual(
          await page.evaluate(
            async ({ context, id }) =>
              window.play.workspace({
                kind: "run_detail",
                context,
                run_id: id,
              }),
            { context: originalContext, id: savedRun.id },
          ),
          savedRun,
        );
    }
    const final = await current(page);
    const finalBlocks = final.exploration.stage.blocks;
    await capture(application, "native-ordered-notebook");
    await create(page, "Exploration", "Independent order");
    await expect(notebook(page).getByRole("article")).toHaveCount(1);
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "",
    );
    await explorer(page)
      .getByRole("button", { name: "Clue order", exact: true })
      .click();
    await expect
      .poll(async () =>
        (await current(page)).exploration.stage.blocks.map((block) => block.id),
      )
      .toEqual(finalIds);
    await application.close();
    application = await launch();
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(notebook(page)).toBeVisible();
    const reopened = await current(page);
    assert.deepEqual(reopened.exploration.stage.blocks, finalBlocks);
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Reasoning stays with this Block.",
    );
    if (process.platform === "darwin")
      await expect(firstCell()).toContainText("output: First.");
    assert.deepEqual(await readFile(fixture), sourceBefore);
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

test("archive frees an active slot while retained evidence and restore limits stay scoped", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-archive-slots-"));
  const application = await electron.launch({
    args: [main, `--user-data-dir=${profile}`],
  });
  try {
    const page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Bounded notebooks");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Original study");
    const initial = await current(page);
    const context = {
      workspace_id: initial.workspace.id,
      exploration_id: initial.exploration.id,
      stage_id: initial.exploration.stage.id,
    };
    await page.evaluate(async (context) => {
      for (let index = 0; index < 7; index++)
        await window.play.workspace({
          kind: "add_text",
          context,
          key: crypto.randomUUID(),
          title: `Clue ${index}`,
          text: "Gur xrl.",
        });
    }, context);
    await page.reload();
    await expect(notebook(page)).toBeVisible();
    await expect(
      notebook(page).getByRole("button", { name: "Add text", exact: true }),
    ).toBeDisabled();
    await page
      .getByLabel("Stage note", { exact: true })
      .fill("Keep this draft while making room.");
    const original = notebook(page).getByRole("article", {
      name: "Clue 0 block",
      exact: true,
    });
    await original
      .getByRole("button", { name: "Archive Clue 0", exact: true })
      .click();
    await expect(original).toHaveCount(0);
    await expect(
      notebook(page).getByRole("button", { name: "Add text", exact: true }),
    ).toBeEnabled();
    await notebook(page)
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    const dialog = page.getByRole("dialog", {
      name: "Add text block",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Replacement");
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Replacement clue");
    await dialog.getByRole("button", { name: "Add text", exact: true }).click();
    await notebook(page)
      .getByRole("button", { name: "Archived blocks (1)", exact: true })
      .click();
    let archive = page.getByRole("region", {
      name: "Archived blocks",
      exact: true,
    });
    await expect(
      archive.getByRole("button", { name: "Restore Clue 0", exact: true }),
    ).toBeDisabled();
    await expect(archive).toContainText("8 / 8 active");
    const replacement = notebook(page).getByRole("article", {
      name: "Replacement block",
      exact: true,
    });
    await replacement
      .getByRole("button", { name: "Archive Replacement", exact: true })
      .click();
    await expect(replacement).toHaveCount(0);
    archive = page.getByRole("region", {
      name: "Archived blocks",
      exact: true,
    });
    const restore = archive.getByRole("button", {
      name: "Restore Clue 0",
      exact: true,
    });
    await expect(restore).toBeEnabled();
    await restore.click();
    await expect(original).toBeFocused();
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Keep this draft while making room.",
    );
    const saved = (await current(page)).exploration;
    assert.equal(saved.stage.blocks.length, 8);
    assert.equal(saved.stage.archivedBlocks.length, 1);
    assert.equal(
      saved.stage.archivedBlocks[0]?.block.kind === "asset"
        ? saved.stage.archivedBlocks[0].block.asset.title
        : null,
      "Replacement",
    );
    await create(page, "Exploration", "Independent study");
    await expect(
      notebook(page).getByRole("button", {
        name: "Archived blocks (0)",
        exact: true,
      }),
    ).toBeDisabled();
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "",
    );
    await explorer(page)
      .getByRole("button", { name: "Original study", exact: true })
      .click();
    await expect(notebook(page)).toContainText("Clue 0");
    assert.deepEqual((await current(page)).exploration, saved);
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "Keep this draft while making room.",
    );
    await page
      .getByRole("button", { name: "Save reasoning", exact: true })
      .click();
    await expect(
      page.getByRole("status", { name: "Note save status" }),
    ).toHaveText("Saved");
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
