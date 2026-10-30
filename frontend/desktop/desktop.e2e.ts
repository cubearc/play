import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  cp,
  mkdir,
  mkdtemp,
  readFile,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
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
import axe from "axe-core";

declare global {
  interface Window {
    axe: typeof axe;
  }
}

import type {} from "./bridge.ts";
import { nativeFiles } from "./native-files.ts";
import { type Context, reasoningOf } from "./workspace-contract.ts";

const nativeWorkers = ["darwin", "linux"].includes(process.platform);

test("native terminal connects on supported platforms and refuses unavailable policies", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-pty-ui-"));
  let application = await electron.launch({
    chromiumSandbox: true,
    args: [main, `--user-data-dir=${profile}`],
  });
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    const errors: string[] = [];
    let probingAccessibility = false;
    page.on("console", (message) => {
      if (message.type() === "error" && !probingAccessibility)
        errors.push(message.text());
    });
    page.on("pageerror", (error) => errors.push(error.message));
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Contextual terminal");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Follow the clue");
    await notebook(page)
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    const dialog = page.getByRole("dialog", {
      name: "Add text block",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Cipher");
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Uryyb");
    await dialog.getByRole("button", { name: "Add text", exact: true }).click();
    const snapshot = await current(page);
    const trail = snapshot.exploration.trails[0];
    assert.ok(trail?.pipeline);
    const context = {
      workspace_id: snapshot.workspace.id,
      exploration_id: snapshot.exploration.id,
      stage_id: snapshot.exploration.stage.id,
    };
    const history = () =>
      page.evaluate(
        ({ context, trail_id }) =>
          window.play.workspace({
            kind: "list_runs",
            context,
            trail_id,
            before: null,
          }),
        { context, trail_id: trail.id },
      );
    await notebook(page)
      .getByRole("button", { name: "Explore Cipher", exact: true })
      .click();
    // The existing VSCode textarea resizer attempts a blocked data URL before
    // this feature opens. Start the terminal-specific console check here.
    errors.length = 0;
    await page
      .getByRole("button", { name: "Toggle Cube terminal", exact: true })
      .click();
    const panel = page.getByRole("region", {
      name: "Cube terminal",
      exact: true,
    });
    if (!nativeWorkers) {
      await expect(panel.getByRole("status")).toHaveText("Unavailable");
      await expect(panel.getByRole("alert")).toContainText("requires macOS");
      await expect(
        panel.getByRole("button", { name: "Insert selected Run", exact: true }),
      ).toBeDisabled();
      assert.equal((await history()).runs.length, 0);
      return;
    }
    await expect(panel.getByRole("status")).toHaveText("Connected");
    const input = panel.locator(".xterm-helper-textarea");
    await input.pressSequentially("size");
    await input.press("Enter");
    await expect(panel.locator(".xterm-rows")).toContainText("columns");
    await panel
      .getByRole("button", { name: "Insert Asset", exact: true })
      .click();
    await input.press("Enter");
    await expect(panel.locator(".xterm-rows")).toContainText('"text": "Uryyb"');
    await panel
      .getByRole("button", { name: "Insert selected Run", exact: true })
      .click();
    assert.equal(
      (await history()).runs.length,
      0,
      "inserting a command must not execute it",
    );
    await input.press("Enter");
    await expect
      .poll(async () => (await history()).runs[0]?.status)
      .toBe("completed");
    const run = (await history()).runs[0];
    assert.ok(run);
    assert.equal(run.caller, "cli");
    const detail = await frozenRun(page, context, run.id);
    assert.match(JSON.stringify(detail.result?.outputs), /Hello/);
    await expect(
      page.getByRole("region", { name: "Trail canvas", exact: true }),
    ).toContainText("Hello");
    await input.pressSequentially(`run status ${run.id}`);
    await input.press("Enter");
    await expect(panel.locator(".xterm-rows")).toContainText("Hello");
    await page
      .getByRole("region", { name: "Cell result", exact: true })
      .scrollIntoViewIfNeeded();
    await capture(application, "native-context-terminal");
    assert.deepEqual(errors, [], "before accessibility probe");
    // axe inserts measurement styles; product CSP deliberately rejects them.
    probingAccessibility = true;
    assert.deepEqual((await accessibility(page)).violations, []);
    probingAccessibility = false;
    for (const settings of [
      { width: 800, height: 540, zoom: 1 },
      { width: 1280, height: 800, zoom: 2 },
    ]) {
      await application.evaluate(({ BrowserWindow }, s) => {
        const w = BrowserWindow.getAllWindows().find((w) => w.isVisible());
        if (!w) throw new Error("window_missing");
        w.setSize(s.width, s.height);
        w.webContents.setZoomFactor(s.zoom);
      }, settings);
      await expect
        .poll(() =>
          panel.evaluate((element) => ({
            shell: document.documentElement.scrollWidth > innerWidth,
            panel: element.scrollWidth > element.clientWidth + 1,
          })),
        )
        .toEqual({ shell: false, panel: false });
      await expect(
        panel.getByRole("button", { name: "Close terminal", exact: true }),
      ).toBeInViewport();
    }
    await panel.getByRole("button", { name: "Reconnect", exact: true }).click();
    await expect(panel.getByRole("status")).toHaveText("Connected");
    await panel
      .getByRole("button", { name: "Close terminal", exact: true })
      .click();
    await expect(panel).toBeHidden();
    assert.deepEqual(errors, []);
    await application.close();
    application = await electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
    page = await application.firstWindow();
    await expect(notebook(page)).toBeVisible();
    assert.deepEqual(await frozenRun(page, context, run.id), detail);
    await expect(
      page.getByRole("region", { name: "Cube terminal", exact: true }),
    ).toBeHidden();
  } finally {
    await application.close();
    await rm(profile, { recursive: true, force: true });
  }
});

test("Core registrations drive spiral controls, named outputs and unrelated text execution", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-native-registry-"));
  let application = await electron.launch({
    chromiumSandbox: true,
    args: [main, `--user-data-dir=${profile}`],
  });
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Read the spiral");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Follow the samples");
    const image = fileURLToPath(
      new URL("../../tests/fixtures/spiral.png", import.meta.url),
    );
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, image);
    await notebook(page)
      .getByRole("button", { name: "Add image", exact: true })
      .click();
    let editor = page.getByRole("dialog", {
      name: "Add image with caption",
      exact: true,
    });
    await editor
      .getByRole("textbox", { name: "Caption", exact: true })
      .fill("Read the grayscale bytes clockwise, then decode the text.");
    await editor
      .getByRole("button", { name: "Choose PNG", exact: true })
      .click();
    await notebook(page)
      .getByRole("button", { name: "Explore spiral.png", exact: true })
      .click();
    const canvas = page.getByRole("region", {
      name: "Trail canvas",
      exact: true,
    });
    await canvas
      .getByRole("button", { name: "New Pipeline", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "New Pipeline", exact: true });
    await editor
      .getByRole("textbox", { name: "Pipeline name", exact: true })
      .fill("Spiral to text");
    const choose = async (label: string, option: string) => {
      await editor.getByRole("combobox", { name: label, exact: true }).click();
      await editor.getByRole("option", { name: option, exact: true }).click();
    };
    await choose("Operation", "image pixels");
    await choose("Traversal", "Spiral clockwise");
    await choose("Start corner", "Top left");
    const channels = editor.getByRole("checkbox", {
      name: "Use channels",
      exact: true,
    });
    await channels.focus();
    await page.keyboard.press("Space");
    await expect(
      editor.getByRole("button", { name: "Create Pipeline", exact: true }),
    ).toBeDisabled();
    await editor
      .getByRole("textbox", { name: "Channels (JSON)", exact: true })
      .fill('["gray"]');
    await expect(
      editor.getByRole("button", { name: "Create Pipeline", exact: true }),
    ).toBeEnabled();
    await expect(
      editor.getByRole("button", { name: "Create Pipeline", exact: true }),
    ).toBeInViewport();
    await capture(application, "native-spiral-editor");
    await application.evaluate(({ BrowserWindow }) => {
      const window = BrowserWindow.getAllWindows()[0];
      if (!window) throw new Error("missing_window");
      window.setSize(800, 540);
      window.webContents.setZoomFactor(2);
    });
    await expect(
      editor.getByRole("button", { name: "Create Pipeline", exact: true }),
    ).toBeInViewport();
    await expect
      .poll(() =>
        page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth,
        ),
      )
      .toBe(true);
    await capture(application, "native-spiral-compact");
    await application.evaluate(({ BrowserWindow }) => {
      const window = BrowserWindow.getAllWindows()[0];
      if (!window) throw new Error("missing_window");
      window.webContents.setZoomFactor(1);
      window.setSize(1280, 800);
    });

    assert.deepEqual((await accessibility(page)).violations, []);
    const add = async (operation: string) => {
      await editor
        .getByRole("button", { name: "Add node", exact: true })
        .click();
      await choose("Operation", operation);
    };
    await add("Decode UTF-8");
    await choose("input · bytes", "1. image pixels · data");
    await add("ROT13");
    await choose("input · text", "2. Decode UTF-8 · output");
    await add("text reverse");
    await choose("input · text", "3. ROT13 · output");
    await add("text reverse");
    await choose("input · text", "4. text reverse · output");
    await editor
      .getByRole("button", { name: "Create Pipeline", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(nativeWorkers ? "output: The key!!" : "Execution rejected");
    if (!nativeWorkers) return;
    const original = await latestRun(page, "spiral.png");
    assert.equal(original.detail.result?.outputs?.output, "The key!!");
    assert.equal(original.detail.attempts.length, 5);

    const ports = original.detail.attempts[0]?.result?.outputs;
    assert.equal(
      ports?.data && (ports.data as { base64: string }).base64,
      Buffer.from("Gur xrl!!").toString("base64"),
    );
    assert.ok(ports);
    assert.equal(
      (ports.output as { traversal: string }).traversal,
      "spiral_cw",
    );
    await capture(application, "native-spiral-canvas", false);
    assert.deepEqual((await accessibility(page)).violations, []);
    await application.close();
    application = await electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    const restored = await latestRun(page, "spiral.png");
    assert.deepEqual(restored, original);
  } finally {
    await application.evaluate(({ app }) => app.exit(0));
    await application.close();
    await rm(profile, { recursive: true, force: true });
  }
});

test("numeric fields compose exact image facts and caption values with retained evidence", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-native-numeric-"));
  let application = await electron.launch({
    chromiumSandbox: true,
    args: [main, `--user-data-dir=${profile}`],
  });
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Measure the clue");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Exact image dimensions");
    const image = fileURLToPath(
      new URL("../../tests/fixtures/composition.png", import.meta.url),
    );
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, image);
    await notebook(page)
      .getByRole("button", { name: "Add image", exact: true })
      .click();
    let editor = page.getByRole("dialog", {
      name: "Add image with caption",
      exact: true,
    });
    await editor
      .getByRole("textbox", { name: "Caption", exact: true })
      .fill("7");
    await editor
      .getByRole("button", { name: "Choose PNG", exact: true })
      .click();
    await notebook(page)
      .getByRole("button", { name: "Explore composition.png", exact: true })
      .click();
    const canvas = page.getByRole("region", {
      name: "Trail canvas",
      exact: true,
    });
    await canvas
      .getByRole("button", { name: "New Pipeline", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "New Pipeline", exact: true });
    await editor
      .getByRole("textbox", { name: "Pipeline name", exact: true })
      .fill("Dimensions plus score");
    const choose = async (label: string, option: string) => {
      await editor.getByRole("combobox", { name: label, exact: true }).click();
      await editor.getByRole("option", { name: option, exact: true }).click();
    };
    await editor
      .getByRole("textbox", { name: "Input name", exact: true })
      .fill("caption");
    await editor
      .getByRole("button", { name: "Add input", exact: true })
      .click();
    await choose("Asset for caption", "Image caption · r1");
    await choose("Operation", "Image dimensions");
    const add = async (operation: string) => {
      await editor
        .getByRole("button", { name: "Add node", exact: true })
        .click();
      await choose("Operation", operation);
    };
    await add("Select field");
    await choose("input · json", "1. Image dimensions · output");
    await editor
      .getByRole("textbox", { name: "Field path", exact: true })
      .fill("/dimensions");
    await add("Maximum");
    await choose("input · json", "2. Select field · output");
    await add("Parse integer");
    await choose("input · text", "caption · text");
    await add("Add");
    await choose("left · json", "3. Maximum · output");
    await choose("right · json", "Literal · json");
    const integerValue = editor.getByRole("textbox", {
      name: "Integer value",
      exact: true,
    });
    await integerValue.fill("9007199254740990.5");
    await expect(
      editor.getByRole("button", { name: "Create Pipeline", exact: true }),
    ).toBeDisabled();
    await editor
      .getByRole("button", { name: "Node 3: Maximum", exact: true })
      .click();
    await editor
      .getByRole("button", { name: "Node 5: Add", exact: true })
      .click();
    await expect(integerValue).toHaveValue("9007199254740990.5");
    await integerValue.fill("7");
    await choose("right · json", "4. Parse integer · output");
    await capture(application, "native-numeric-editor");
    await editor
      .getByRole("button", { name: "Create Pipeline", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(nativeWorkers ? "output: 8" : "Execution rejected");
    await page
      .getByRole("button", { name: "Toggle Primary Side Bar", exact: true })
      .click();
    await canvas
      .getByRole("region", { name: "Pipeline graph", exact: true })
      .evaluate((element) => element.scrollIntoView({ block: "start" }));
    await capture(application, "native-numeric-canvas", false);
    await page
      .getByRole("button", { name: "Toggle Primary Side Bar", exact: true })
      .click();
    assert.deepEqual((await accessibility(page)).violations, []);
    if (!nativeWorkers) return;
    const original = await latestRun(page, "composition.png");
    assert.equal(original.detail.result?.outputs?.output, 8);
    assert.equal(original.detail.attempts.length, 5);
    assert.equal(original.detail.caller, "desktop");
    await canvas
      .getByRole("button", { name: "Run with Cube CLI", exact: true })
      .click();
    await expect
      .poll(
        async () => (await latestRun(page, "composition.png")).detail.caller,
      )
      .toBe("cli");
    await expect
      .poll(async () => {
        const { detail } = await latestRun(page, "composition.png");
        return { status: detail.status, result: detail.result };
      })
      .toEqual({ status: "completed", result: original.detail.result });
    const cliRun = await latestRun(page, "composition.png");
    assert.notEqual(cliRun.detail.id, original.detail.id);
    assert.deepEqual(cliRun.detail.plan.inputs, original.detail.plan.inputs);
    assert.deepEqual(
      cliRun.detail.plan.pipeline,
      original.detail.plan.pipeline,
    );
    const inspected = await page.evaluate(
      async ({ context, id, pipeline, revision }) => ({
        run: await window.play.cli({ kind: "status", context, run_id: id }),
        pipeline: await window.play.cli({
          kind: "pipeline",
          context,
          pipeline_id: pipeline,
          revision,
        }),
        cancelled: await window.play.cli({
          kind: "cancel",
          context,
          run_id: id,
        }),
      }),
      {
        context: cliRun.context,
        id: cliRun.detail.id,
        pipeline: cliRun.detail.pipelineId,
        revision: cliRun.detail.pipelineRevision,
      },
    );
    const assetReads = await page.evaluate(
      async ({ context, inputs }) => {
        const values = [];
        for (const [name, input] of Object.entries(inputs)) {
          if ("assetId" in input)
            values.push({
              name,
              asset: await window.play.cli({
                kind: "asset",
                context,
                asset_id: input.assetId,
                revision: input.revision,
              }),
            });
        }
        return values;
      },
      { context: cliRun.context, inputs: cliRun.detail.plan.inputs },
    );
    assert.equal(
      assetReads.find((v) => v.name === "source")?.asset.content.kind,
      "image",
    );
    assert.deepEqual(
      assetReads.find((v) => v.name === "caption")?.asset.content,
      { kind: "text", text: "7" },
    );
    assert.deepEqual(inspected.run, cliRun.detail);
    assert.deepEqual(inspected.cancelled, cliRun.detail);
    assert.deepEqual(
      inspected.pipeline.definition,
      cliRun.detail.plan.pipeline.definition,
    );
    await canvas
      .locator("summary")
      .filter({ hasText: "Saved Run & input evidence" })
      .click();
    await expect(canvas).toContainText("Cube CLI");
    await canvas
      .locator(".cell-code")
      .evaluate((element) => element.scrollIntoView({ block: "start" }));
    await capture(application, "native-context-cli", false);
    await pipelineReflow(application, true);

    await canvas
      .getByRole("button", { name: "Edit Pipeline", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "Edit Pipeline", exact: true });
    await choose("right · json", "Literal · json");
    await editor
      .getByRole("textbox", { name: "Integer value", exact: true })
      .fill("9007199254740991");
    await editor
      .getByRole("button", { name: "Save revision", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText("integer exceeds the exact range");
    const failed = await latestRun(page, "composition.png");
    assert.equal(failed.detail.result?.code, "numeric_overflow");
    assert.equal(failed.detail.attempts.length, 4);
    await application.close();
    application = await electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(notebook(page)).toBeVisible();
    assert.deepEqual(
      await frozenRun(page, original.context, original.detail.id),
      original.detail,
    );
    assert.deepEqual(
      await frozenRun(page, cliRun.context, cliRun.detail.id),
      cliRun.detail,
    );
    assert.deepEqual(
      await frozenRun(page, failed.context, failed.detail.id),
      failed.detail,
    );
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

test("structured fields preserve JSON types, distinguish null and missing, and retain failed paths", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-native-json-"));
  let application = await electron.launch({
    chromiumSandbox: true,
    args: [main, `--user-data-dir=${profile}`],
  });
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Structured evidence");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Choose the exact field");
    await notebook(page)
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    let editor = page.getByRole("dialog", {
      name: "Add text block",
      exact: true,
    });
    await editor
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Structured clue");
    await editor
      .getByRole("textbox", { name: "Text", exact: true })
      .fill(
        '{"clue":"Gur xrl.","empty":null,"ratio":1.5,"payload":{"base64":"YQ=="}}',
      );
    await editor.getByRole("button", { name: "Add text", exact: true }).click();
    await notebook(page)
      .getByRole("button", { name: "Explore Structured clue", exact: true })
      .click();
    const canvas = page.getByRole("region", {
      name: "Trail canvas",
      exact: true,
    });
    await canvas
      .getByRole("button", { name: "New Pipeline", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "New Pipeline", exact: true });
    await pipelineReflow(application);
    const choose = async (label: string, option: string) => {
      await editor.getByRole("combobox", { name: label, exact: true }).click();
      await editor.getByRole("option", { name: option, exact: true }).click();
    };
    const add = async (operation: string) => {
      await editor
        .getByRole("button", { name: "Add node", exact: true })
        .click();
      await choose("Operation", operation);
    };
    await editor
      .getByRole("textbox", { name: "Pipeline name", exact: true })
      .fill("Decode a structured clue");
    await choose("Operation", "Parse JSON");
    await add("Select field");
    await choose("input · json", "1. Parse JSON · output");
    await editor
      .getByRole("textbox", { name: "Field path", exact: true })
      .fill("/clue");
    await add("Use text value");
    await choose("input · json", "2. Select field · output");
    await add("ROT13");
    await choose("input · text", "3. Use text value · output");
    await editor
      .getByRole("button", { name: "Create Pipeline", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(nativeWorkers ? "output: The key." : "Execution rejected");
    if (!nativeWorkers) return;
    const original = await latestRun(page, "Structured clue");
    assert.equal(original.detail.attempts.length, 4);
    const revise = async (pointer: string) => {
      await canvas
        .getByRole("button", { name: "Edit Pipeline", exact: true })
        .click();
      editor = page.getByRole("dialog", { name: "Edit Pipeline", exact: true });
      await editor
        .getByRole("button", { name: "Node 2: Select field", exact: true })
        .click();
      await editor
        .getByRole("textbox", { name: "Field path", exact: true })
        .fill(pointer);
      await editor
        .getByRole("button", { name: "Save revision", exact: true })
        .click();
      await expect(editor).toBeHidden();
    };
    await revise("/payload");
    await canvas
      .getByRole("button", { name: "Node 2: Select field", exact: true })
      .click();
    await canvas.getByRole("button", { name: "Run node", exact: true }).click();
    const result = canvas.getByRole("region", { name: "Cell result" });
    await expect(result.getByRole("status")).toContainText('"base64": "YQ=="');
    await expect(result.getByRole("status")).not.toContainText("Base64 YQ==");
    const object = await latestRun(page, "Structured clue");
    assert.deepEqual(object.detail.result?.outputs?.output, { base64: "YQ==" });
    assert.equal(object.detail.attempts.length, 2);
    await revise("/missing");
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(result).toContainText(
      "selected field or array index does not exist",
    );
    const missing = await latestRun(page, "Structured clue");
    assert.equal(missing.detail.result?.code, "missing_field");
    assert.equal(missing.detail.attempts.length, 2);
    await revise("/empty");
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(result).toContainText("selected value is not text");
    const empty = await latestRun(page, "Structured clue");
    assert.equal(empty.detail.result?.code, "invalid_type");
    assert.equal(empty.detail.attempts.length, 3);
    assert.equal(empty.detail.attempts[1]?.result?.outputs?.output, null);
    await result
      .getByText("Saved Run & input evidence", { exact: false })
      .click();
    await expect(result).toContainText("output: null");
    assert.deepEqual((await accessibility(page)).violations, []);
    await capture(application, "native-structured-failure");
    await application.close();
    application = await electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(notebook(page)).toBeVisible();
    for (const saved of [original, object, missing, empty])
      assert.deepEqual(
        await frozenRun(page, saved.context, saved.detail.id),
        saved.detail,
      );
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

test("typed composition joins image/caption branches and keeps binary and failed-decode evidence", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-native-composition-"));
  let application = await electron.launch({
    chromiumSandbox: true,
    args: [main, `--user-data-dir=${profile}`],
  });
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Compose the clues");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Read the hidden bytes");
    const image = fileURLToPath(
      new URL("../../tests/fixtures/composition.png", import.meta.url),
    );
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, image);
    await notebook(page)
      .getByRole("button", { name: "Add image", exact: true })
      .click();
    let editor = page.getByRole("dialog", {
      name: "Add image with caption",
      exact: true,
    });
    await editor
      .getByRole("textbox", { name: "Caption", exact: true })
      .fill("xrl.");
    await editor
      .getByRole("button", { name: "Choose PNG", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await notebook(page)
      .getByRole("button", { name: "Explore composition.png", exact: true })
      .click();
    const canvas = page.getByRole("region", {
      name: "Trail canvas",
      exact: true,
    });
    await canvas
      .getByRole("button", { name: "New Pipeline", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "New Pipeline", exact: true });
    await editor
      .getByRole("textbox", { name: "Pipeline name", exact: true })
      .fill("Joined clues");
    await editor
      .getByRole("textbox", { name: "Input name", exact: true })
      .fill("caption");
    await editor
      .getByRole("button", { name: "Add input", exact: true })
      .click();
    const choose = async (label: string, option: string) => {
      await editor.getByRole("combobox", { name: label, exact: true }).click();
      await editor.getByRole("option", { name: option, exact: true }).click();
    };
    await choose("Asset for caption", "Image caption · r1");
    const add = async (operation: string) => {
      await editor
        .getByRole("button", { name: "Add node", exact: true })
        .click();
      await choose("Operation", operation);
    };
    await add("Decode UTF-8");
    await choose("input · bytes", "1. image pixel · output");
    await add("ROT13");
    await choose("input · text", "2. Decode UTF-8 · output");
    await add("ROT13");
    await choose("input · text", "caption · text");
    await add("Concatenate text");
    await choose("left · text", "3. ROT13 · output");
    await choose("right · text", "4. ROT13 · output");
    await editor
      .getByRole("textbox", { name: "Separator", exact: true })
      .fill(" ");
    await capture(application, "native-composition-editor");
    await editor
      .getByRole("button", { name: "Create Pipeline", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(nativeWorkers ? "output: The key." : "Execution rejected");
    if (!nativeWorkers) return;
    await expect(
      canvas.getByRole("button", { name: "Run cell", exact: true }),
    ).not.toHaveAttribute("disabled", "");
    await page
      .getByRole("button", { name: "Toggle Primary Side Bar", exact: true })
      .click();
    await expect(explorer(page)).toBeHidden();
    await canvas
      .getByRole("region", { name: "Pipeline graph", exact: true })
      .evaluate((e) => e.scrollIntoView({ block: "start" }));
    await capture(application, "native-composition-canvas", false);
    await page
      .getByRole("button", { name: "Toggle Primary Side Bar", exact: true })
      .click();
    assert.deepEqual((await accessibility(page)).violations, []);
    const first = await current(page);
    const trail = first.exploration.trails.find(
      (t) => t.source.content.kind === "image",
    );
    assert.ok(trail);
    const details = async () =>
      page.evaluate(async (t) => {
        const context = {
          workspace_id: t.workspaceId,
          exploration_id: t.explorationId,
          stage_id: t.stageId,
        };
        const runs = await window.play.workspace({
          kind: "list_runs",
          context,
          trail_id: t.id,
          before: null,
        });
        const latest = runs.runs[0];
        if (!latest) throw new Error("run_missing");
        return window.play.workspace({
          kind: "run_detail",
          context,
          run_id: latest.id,
        });
      }, trail);
    const joined = await details();
    assert.equal(joined.attempts.length, 5);
    assert.ok(joined.attempts.every((a) => a.status === "completed"));
    assert.equal(joined.result?.outputs?.output, "The key.");
    await canvas
      .getByRole("button", { name: "Save output as block", exact: true })
      .click();
    await expect(
      notebook(page).getByRole("article", {
        name: "Concatenate text · output · Run 1 block",
        exact: true,
      }),
    ).toContainText("The key.");
    await notebook(page)
      .getByRole("button", { name: "Explore composition.png", exact: true })
      .click();
    await canvas
      .getByRole("button", { name: "New Pipeline", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "New Pipeline", exact: true });
    await editor
      .getByRole("textbox", { name: "Pipeline name", exact: true })
      .fill("Binary join");
    await expect(
      editor.getByRole("button", { name: /^Node [0-9]+:/ }),
    ).toHaveCount(1);
    await choose("Operation", "Encode UTF-8");
    await editor
      .getByRole("textbox", { name: "Text value", exact: true })
      .fill("é");
    await add("Concatenate bytes");
    await choose("left · bytes", "1. Encode UTF-8 · output");
    const encoded = editor.getByRole("textbox", {
      name: "Bytes (Base64)",
      exact: true,
    });
    await encoded.fill("/w==");
    await editor
      .getByRole("button", { name: "Create Pipeline", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText("w6n/");
    await expect(
      canvas.getByRole("button", { name: "Run cell", exact: true }),
    ).not.toHaveAttribute("disabled", "");
    await canvas
      .getByRole("button", { name: "Edit Pipeline", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "Edit Pipeline", exact: true });
    await add("Decode UTF-8");
    await choose("input · bytes", "2. Concatenate bytes · output");
    await add("Concatenate text");
    await choose("left · text", "3. Decode UTF-8 · output");
    await editor
      .getByRole("button", { name: "Save revision", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText("not valid UTF-8");
    const failed = await details();
    assert.equal(failed.status, "failed");
    assert.equal(failed.attempts.length, 3);
    assert.equal(failed.attempts[0]?.status, "completed");
    assert.equal(failed.attempts[1]?.status, "completed");
    assert.equal(failed.attempts[2]?.status, "failed");
    await application.close();
    application = await electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    const reopened = await current(page);
    assert.ok(
      reopened.exploration.stage.blocks.some(
        (b) =>
          b.kind === "asset" &&
          b.asset.content.kind === "text" &&
          b.asset.content.text === "The key.",
      ),
    );
    const restored = await page.evaluate(
      ({ t, id }) =>
        window.play.workspace({
          kind: "run_detail",
          context: {
            workspace_id: t.workspaceId,
            exploration_id: t.explorationId,
            stage_id: t.stageId,
          },
          run_id: id,
        }),
      { t: trail, id: joined.id },
    );
    assert.deepEqual(restored, joined);
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

test("named assets keep figures grouped and share a graph without sharing bindings or Run targets", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-native-inputs-"));
  let application = await electron.launch({
    chromiumSandbox: true,
    args: [main, `--user-data-dir=${profile}`],
  });
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Image and caption study");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "First interpretation");
    const addFigure = async (caption: string) => {
      await application.evaluate(({ dialog }, path) => {
        dialog.showOpenDialog = async () => ({
          canceled: false,
          filePaths: [path],
        });
      }, fixture);
      await notebook(page)
        .getByRole("button", { name: "Add image", exact: true })
        .click();
      const dialog = page.getByRole("dialog", {
        name: "Add image with caption",
        exact: true,
      });
      await dialog
        .getByRole("textbox", { name: "Caption", exact: true })
        .fill(caption);
      await dialog
        .getByRole("button", { name: "Choose PNG", exact: true })
        .click();
      await expect(dialog).toBeHidden();
      await notebook(page)
        .getByRole("button", { name: "Explore sample.png", exact: true })
        .click();
    };
    await addFigure("Gur xrl.");
    let canvas = page.getByRole("region", {
      name: "Trail canvas",
      exact: true,
    });
    await canvas
      .getByRole("button", { name: "New Pipeline", exact: true })
      .click();
    let editor = page.getByRole("dialog", {
      name: "New Pipeline",
      exact: true,
    });
    await editor
      .getByRole("textbox", { name: "Pipeline name", exact: true })
      .fill("Image and caption");
    await editor
      .getByRole("textbox", { name: "Input name", exact: true })
      .fill("caption");
    await editor
      .getByRole("button", { name: "Add input", exact: true })
      .click();
    await expect(
      editor.getByRole("button", { name: "Create Pipeline", exact: true }),
    ).toHaveAttribute("disabled", "");
    await editor
      .getByRole("combobox", { name: "Asset for caption", exact: true })
      .click();
    await editor
      .getByRole("option")
      .filter({ hasText: /caption.*r1/i })
      .click();
    await editor.getByRole("button", { name: "Add node", exact: true }).click();
    await editor
      .getByRole("combobox", { name: "Operation", exact: true })
      .click();
    await editor.getByRole("option", { name: "ROT13", exact: true }).click();
    await expect(
      editor.getByRole("combobox", { name: "input · text", exact: true }),
    ).toContainText("caption · text");
    await capture(application, "native-pipeline-inputs-editor");
    await editor
      .getByRole("button", { name: "Create Pipeline", exact: true })
      .click();
    await expect(editor).toBeHidden();
    const first = await current(page);
    const t = first.exploration.trails.find(
      (t) => t.source.content.kind === "image",
    );
    assert.ok(t?.pipeline);
    const run = async (target: "default" | "image", expected: string) => {
      if (target === "image") {
        await canvas.getByRole("button", { name: /Node 1:/ }).click();
        await canvas
          .getByRole("button", { name: "Run node", exact: true })
          .click();
      } else
        await canvas
          .getByRole("button", { name: "Run cell", exact: true })
          .click();
      await expect(
        canvas.getByRole("region", { name: "Cell result" }),
      ).toContainText(nativeWorkers ? expected : "Execution rejected");
    };
    await run("default", "output: The key.");
    await run("image", "FFAy");
    await canvas
      .getByRole("region", { name: "Bound Pipeline inputs" })
      .evaluate((element) => element.scrollIntoView({ block: "start" }));
    await capture(application, "native-pipeline-inputs-canvas", false);
    assert.deepEqual((await accessibility(page)).violations, []);
    const runs = await page.evaluate(
      async ({ workspaceId, explorationId, stageId, id }) =>
        window.play.workspace({
          kind: "list_runs",
          context: {
            workspace_id: workspaceId,
            exploration_id: explorationId,
            stage_id: stageId,
          },
          trail_id: id,
          before: null,
        }),
      t,
    );
    assert.equal(runs.runs.length, 2);
    assert.equal(runs.runs[0]?.operation.operation, "image.pixels.read");
    assert.equal(runs.runs[1]?.operation.operation, "cipher.rot13");
    await page.getByRole("tab", { name: "Stage 1", exact: true }).click();
    const figure = notebook(page).getByRole("article", {
      name: "sample.png block",
      exact: true,
    });
    await expect(figure).toContainText("Gur xrl.");
    await expect(figure.locator("img")).toHaveCount(1);
    await figure
      .getByRole("button", { name: "Edit caption", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "Edit caption", exact: true });
    await editor
      .getByRole("textbox", { name: "Caption", exact: true })
      .fill("Frpbaq pyhr.");
    await editor
      .getByRole("button", { name: "Save changes", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await figure
      .getByRole("button", { name: "Explore sample.png", exact: true })
      .click();
    await run("default", "output: The key.");
    await expect(canvas.getByText(/Earlier input ·/)).toHaveCount(0);
    await canvas
      .getByRole("button", { name: "Edit Trail", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "Edit Trail", exact: true });
    await editor
      .getByRole("combobox", { name: "Asset for caption", exact: true })
      .click();
    await expect(
      editor.getByRole("option").filter({ hasText: /pinned; current r2/ }),
    ).toBeVisible();
    await capture(application, "native-pipeline-inputs-binding");
    await editor
      .getByRole("option", { name: "Image caption · r2", exact: true })
      .click();
    await editor
      .getByRole("button", { name: "Save Trail", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await run("default", "output: Second clue.");
    await canvas
      .getByRole("button", { name: "Edit Pipeline", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "Edit Pipeline", exact: true });
    await editor
      .getByRole("textbox", { name: "Input name", exact: true })
      .fill("hint");
    await editor
      .getByRole("button", { name: "Add input", exact: true })
      .click();
    await editor
      .getByRole("combobox", { name: "Asset for hint", exact: true })
      .click();
    await editor
      .getByRole("option", { name: "Image caption · r2", exact: true })
      .click();
    await editor
      .getByRole("button", { name: "Save revision", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await create(page, "Exploration", "Second interpretation");
    await addFigure("Nabgure pyhr.");
    await canvas
      .getByRole("button", { name: "Edit Trail", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "Edit Trail", exact: true });
    await editor
      .getByRole("combobox", { name: "Saved Pipeline", exact: true })
      .click();
    await editor
      .getByRole("option", { name: "Image and caption", exact: true })
      .click();
    await editor
      .getByRole("combobox", { name: "Pipeline revision", exact: true })
      .click();
    await editor
      .getByRole("option", { name: "Revision 1", exact: true })
      .click();
    await expect(
      editor.getByRole("combobox", { name: "Asset for hint", exact: true }),
    ).toHaveCount(0);

    await editor
      .getByRole("combobox", { name: "Asset for caption", exact: true })
      .click();
    await editor
      .getByRole("option")
      .filter({ hasText: /caption.*r1/i })
      .click();
    await editor
      .getByRole("textbox", { name: "Hypothesis", exact: true })
      .fill("A separate reading of the same graph");
    await editor
      .getByRole("button", { name: "Save Trail", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await run("default", "output: Another clue.");
    const second = await current(page);
    const st = second.exploration.trails.find(
      (t) => t.source.content.kind === "image",
    );
    assert.ok(st?.pipeline);
    assert.equal(st.pipeline.id, t.pipeline.id);
    assert.equal(st.pipeline.revision, t.pipeline.revision);
    assert.notEqual(st.bindings.caption?.id, t.bindings.caption?.id);
    await application.close();
    application = await electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(notebook(page)).toBeVisible();
    assert.deepEqual((await current(page)).exploration, second.exploration);
    await notebook(page)
      .getByRole("button", { name: "Explore sample.png", exact: true })
      .click();
    canvas = page.getByRole("region", { name: "Trail canvas", exact: true });
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(nativeWorkers ? "Another clue." : "Execution rejected");
  } finally {
    await application.evaluate(({ dialog }) => {
      dialog.showMessageBox = async () => ({
        response: 1,
        checkboxChecked: false,
      });
    });
    await application.close();
    await rm(profile, { recursive: true, force: true });
  }
});

test("typed graph editing executes dependent nodes and preserves their evidence after restart", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-native-graph-"));
  let application = await electron.launch({
    chromiumSandbox: true,
    args: [main, `--user-data-dir=${profile}`],
  });
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Graph notebook");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Decoding study");
    await notebook(page)
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    const text = page.getByRole("dialog", {
      name: "Add text block",
      exact: true,
    });
    await text
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Graph clue");
    await text
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Gur xrl.");
    await text.getByRole("button", { name: "Add text", exact: true }).click();
    await notebook(page)
      .getByRole("button", { name: "Explore Graph clue", exact: true })
      .click();
    let canvas = page.getByRole("region", {
      name: "Trail canvas",
      exact: true,
    });
    await canvas
      .getByRole("button", { name: "Edit Pipeline", exact: true })
      .click();
    const editor = page.getByRole("dialog", {
      name: "Edit Pipeline",
      exact: true,
    });
    await editor.getByRole("button", { name: "Add node", exact: true }).click();
    await expect(
      editor.getByRole("button", { name: "Node 2: ROT13", exact: true }),
    ).toBeVisible();
    await editor
      .getByRole("button", { name: "Node 2: ROT13", exact: true })
      .focus();
    await page.keyboard.press("ArrowDown");
    await capture(application, "native-pipeline-graph-editor");
    await editor
      .getByRole("button", { name: "Save revision", exact: true })
      .click();
    await expect(editor).toBeHidden();
    const saved = await current(page);
    const trail = saved.exploration.trails[0];
    assert.ok(trail?.pipeline);
    assert.equal(trail.pipeline.definition.nodes.length, 2);
    assert.equal(trail.pipeline.definition.nodes[1]?.position.y, 80);
    assert.equal(
      trail.pipeline.definition.nodes[1]?.inputs.input?.kind,
      "output",
    );
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(
      canvas.getByRole("region", { name: "Cell result", exact: true }),
    ).toContainText(nativeWorkers ? "output: Gur xrl." : "Execution rejected");
    if (nativeWorkers) {
      let evidence = await page.evaluate(async () => {
        const w = await window.play.workspace({ kind: "list" });
        if (!w.lastWorkspaceId) throw new Error("workspace_missing");
        const workspace = await window.play.workspace({
          kind: "view",
          workspace_id: w.lastWorkspaceId,
        });
        if (!workspace.selectedExplorationId)
          throw new Error("exploration_missing");
        const e = await window.play.workspace({
          kind: "open_exploration",
          workspace_id: workspace.id,
          exploration_id: workspace.selectedExplorationId,
          stage_id: workspace.selectedStageId,
        });
        const t = e.trails[0];
        if (!t) throw new Error("trail_missing");
        const context = {
          workspace_id: e.workspaceId,
          exploration_id: e.id,
          stage_id: e.stage.id,
        };
        const runs = await window.play.workspace({
          kind: "list_runs",
          context,
          trail_id: t.id,
          before: null,
        });
        const first = runs.runs[0];
        if (!first) throw new Error("run_missing");
        return window.play.workspace({
          kind: "run_detail",
          context,
          run_id: first.id,
        });
      });
      assert.equal(evidence.attempts.length, 2);
      assert.equal(evidence.attempts[0]?.result?.outputs?.output, "The key.");
      assert.equal(evidence.attempts[1]?.result?.outputs?.output, "Gur xrl.");
      await canvas
        .getByText("Saved Run & input evidence", { exact: false })
        .click();
      await expect(
        canvas.getByText("2 actual node attempt(s)", { exact: false }),
      ).toBeVisible();
      await application.evaluate(({ BrowserWindow }) =>
        BrowserWindow.getAllWindows()
          .find((w) => w.isVisible())
          ?.webContents.setZoomFactor(0.55),
      );
      await capture(application, "native-pipeline-graph-run");
      await application.evaluate(({ BrowserWindow }) =>
        BrowserWindow.getAllWindows()
          .find((w) => w.isVisible())
          ?.webContents.setZoomFactor(1),
      );
      assert.deepEqual((await accessibility(page)).violations, []);
      await canvas
        .getByRole("button", { name: "Save output as block", exact: true })
        .click();
      await expect(
        notebook(page).getByText("Gur xrl.", { exact: true }),
      ).toHaveCount(2);
      const retained = evidence;
      await application.close();
      application = await electron.launch({
        chromiumSandbox: true,
        args: [main, `--user-data-dir=${profile}`],
      });
      page = await application.firstWindow();
      page.setDefaultTimeout(5000);
      await notebook(page)
        .getByRole("button", { name: "Explore Graph clue", exact: true })
        .click();
      canvas = page.getByRole("region", { name: "Trail canvas", exact: true });
      await expect(
        canvas.getByRole("region", { name: "Cell result", exact: true }),
      ).toContainText("output: Gur xrl.");
      evidence = await page.evaluate(
        async ({ context, id }) =>
          window.play.workspace({ kind: "run_detail", context, run_id: id }),
        {
          context: {
            workspace_id: saved.workspace.id,
            exploration_id: saved.exploration.id,
            stage_id: saved.exploration.stage.id,
          },
          id: retained.id,
        },
      );
      assert.deepEqual(evidence, retained);
    }
  } finally {
    await application.close();
    await rm(profile, { recursive: true, force: true });
  }
});

test("explicit Pipeline revisions preserve separate Trail hypotheses and accepted evidence", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-pipeline-revisions-"));
  let application = await electron.launch({
    chromiumSandbox: true,
    args: [main, `--user-data-dir=${profile}`],
  });
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Pipeline notebook");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "First investigation");
    await notebook(page)
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    const dialog = page.getByRole("dialog", {
      name: "Add text block",
      exact: true,
    });
    await dialog
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Encoded clue");
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Gur xrl.");
    await dialog.getByRole("button", { name: "Add text", exact: true }).click();
    await notebook(page)
      .getByRole("button", { name: "Explore Encoded clue", exact: true })
      .click();
    await expect(
      page
        .getByRole("region", { name: "Trail canvas", exact: true })
        .getByRole("button", { name: "Edit Pipeline", exact: true }),
    ).toBeVisible();
    const canvas = page.getByRole("region", {
      name: "Trail canvas",
      exact: true,
    });
    await canvas
      .getByRole("button", { name: "Edit Trail", exact: true })
      .click();
    let editor = page.getByRole("dialog", { name: "Edit Trail", exact: true });
    await editor
      .getByRole("textbox", { name: "Hypothesis", exact: true })
      .fill("The clue uses ROT13.");
    await editor
      .getByRole("button", { name: "Save Trail", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await expect(canvas).toContainText("The clue uses ROT13.");
    await canvas
      .getByRole("button", { name: "New Pipeline", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "New Pipeline", exact: true });
    await editor
      .getByRole("textbox", { name: "Pipeline name", exact: true })
      .fill("Clue transform");
    await editor
      .getByRole("button", { name: "Create Pipeline", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(nativeWorkers ? "output: The key." : "Execution rejected");
    const initial = await current(page);
    const first = initial.exploration.trails[0];
    assert.ok(first?.pipeline);
    await canvas
      .getByRole("button", { name: "Edit Pipeline", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "Edit Pipeline", exact: true });
    await editor
      .getByRole("combobox", { name: "Operation", exact: true })
      .click();
    await editor
      .getByRole("option")
      .filter({ hasText: /brainfuck/i })
      .click();
    await expect(
      editor.getByRole("textbox", { name: "Max instructions", exact: true }),
    ).toBeVisible();
    await expect(
      editor.getByRole("textbox", { name: "Cell bits", exact: true }),
    ).toHaveCount(0);
    await expect(
      editor.getByRole("textbox", { name: "Eof", exact: true }),
    ).toHaveCount(0);
    await editor
      .getByRole("button", { name: "Save revision", exact: true })
      .click();
    await expect(editor).toBeHidden();
    const revised = await current(page);
    assert.equal(revised.exploration.trails[0]?.pipeline?.revision, 2);
    assert.equal(
      revised.exploration.trails[0]?.pipeline?.outputNodeId,
      first.pipeline.outputNodeId,
    );
    assert.equal(
      revised.exploration.trails[0]?.pipeline?.outputOperation,
      "esolang.brainfuck.execute",
    );
    await expect(canvas.getByRole("note")).toContainText(
      "Earlier Pipeline · r1",
    );
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(nativeWorkers ? "output: The key." : "Execution rejected");
    await application.evaluate(({ BrowserWindow }) =>
      BrowserWindow.getAllWindows()
        .find((w) => w.isVisible())
        ?.webContents.setZoomFactor(0.65),
    );
    await capture(application, "native-pipeline-revision");
    await application.evaluate(({ BrowserWindow }) =>
      BrowserWindow.getAllWindows()
        .find((w) => w.isVisible())
        ?.webContents.setZoomFactor(1),
    );
    await canvas
      .getByRole("button", { name: "Fork Pipeline", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "Fork Pipeline", exact: true });
    await expect(
      editor.getByRole("button", { name: "Add node", exact: true }),
    ).toBeDisabled();
    assert.deepEqual((await accessibility(page)).violations, []);
    await editor
      .getByRole("textbox", { name: "Pipeline name", exact: true })
      .fill("Alternative program");
    await editor
      .getByRole("button", { name: "Fork Pipeline", exact: true })
      .click();
    await expect(editor).toBeHidden();
    const forked = await current(page);
    const fork = forked.exploration.trails[0]?.pipeline;
    assert.ok(fork?.origin);
    assert.equal(fork.origin.pipelineId, first.pipeline.id);
    assert.equal(fork.origin.revision, 2);
    await create(page, "Exploration", "Second investigation");
    await notebook(page)
      .getByRole("button", { name: "Add text", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "Add text block", exact: true });
    await editor
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Another clue");
    await editor
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Frpbaq pyhr.");
    await editor.getByRole("button", { name: "Add text", exact: true }).click();
    await notebook(page)
      .getByRole("button", { name: "Explore Another clue", exact: true })
      .click();
    await canvas
      .getByRole("button", { name: "Edit Trail", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "Edit Trail", exact: true });
    await editor
      .getByRole("textbox", { name: "Hypothesis", exact: true })
      .fill("A separate hypothesis for another clue.");
    await editor
      .getByRole("combobox", { name: "Saved Pipeline", exact: true })
      .click();
    await editor
      .getByRole("option", { name: "Clue transform", exact: true })
      .click();
    await editor
      .getByRole("combobox", { name: "Pipeline revision", exact: true })
      .click();
    await editor
      .getByRole("option", { name: "Revision 1", exact: true })
      .click();
    await capture(application, "native-trail-editor");
    await editor
      .getByRole("button", { name: "Save Trail", exact: true })
      .click();
    await expect(editor).toBeHidden();
    const second = await current(page);
    assert.equal(second.exploration.trails[0]?.pipeline?.id, first.pipeline.id);
    assert.equal(second.exploration.trails[0]?.pipeline?.revision, 1);
    assert.notEqual(second.exploration.trails[0]?.source.id, first.source.id);
    assert.deepEqual((await accessibility(page)).violations, []);
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(
      nativeWorkers ? "output: Second clue." : "Execution rejected",
    );
    await canvas
      .getByRole("button", { name: "Edit Trail", exact: true })
      .click();
    editor = page.getByRole("dialog", { name: "Edit Trail", exact: true });
    await editor
      .getByRole("combobox", { name: "Saved Pipeline", exact: true })
      .click();
    await editor
      .getByRole("option", { name: "No Pipeline", exact: true })
      .click();
    await editor
      .getByRole("button", { name: "Save Trail", exact: true })
      .click();
    await expect(editor).toBeHidden();
    await expect(
      canvas.getByRole("button", { name: "Run cell", exact: true }),
    ).toHaveCount(0);
    await expect(
      canvas.getByRole("region", { name: "Cell result" }),
    ).toContainText(
      nativeWorkers ? "output: Second clue." : "Execution rejected",
    );
    await explorer(page)
      .getByRole("button", { name: "First investigation", exact: true })
      .click();
    const restored = await current(page);
    assert.equal(
      restored.exploration.trails[0]?.hypothesis,
      "The clue uses ROT13.",
    );
    assert.equal(restored.exploration.trails[0]?.pipeline?.id, fork.id);
    await application.close();
    application = await electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(notebook(page)).toBeVisible();
    await explorer(page)
      .getByRole("button", { name: "Second investigation", exact: true })
      .click();
    await expect(
      notebook(page).getByRole("region", { name: "Cell result" }),
    ).toContainText(
      nativeWorkers ? "output: Second clue." : "Execution rejected",
    );
    const unbound = await current(page);
    assert.equal(unbound.exploration.trails[0]?.pipeline, null);
    await notebook(page)
      .getByRole("button", { name: "Explore Another clue", exact: true })
      .click();
    const reopened = page.getByRole("region", {
      name: "Trail canvas",
      exact: true,
    });
    await expect(
      reopened.getByRole("region", { name: "Cell result" }),
    ).toContainText(
      nativeWorkers ? "output: Second clue." : "Execution rejected",
    );
    await expect(
      reopened.getByRole("button", { name: "Run cell", exact: true }),
    ).toHaveCount(0);
    assert.deepEqual((await accessibility(page)).violations, []);
    await application.evaluate(({ BrowserWindow }) =>
      BrowserWindow.getAllWindows()
        .find((w) => w.isVisible())
        ?.webContents.setZoomFactor(0.8),
    );
    await capture(application, "native-unbound-history");
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

test("complete notebook keeps Pages inert and runs the selected image through its typed source", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-notebook-complete-"));
  const launch = () =>
    electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
  let application = await launch();
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Complete notebook");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Mixed clues");
    const activate = async (button: ReturnType<Page["getByRole"]>) => {
      await button.focus();
      await page.keyboard.press("Enter");
    };
    await activate(
      notebook(page).getByRole("button", { name: "Add page", exact: true }),
    );
    let dialog = page.getByRole("dialog", { name: "Add page", exact: true });
    await dialog
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Clue page");
    const hostile =
      '<script>globalThis.pageExecuted=true;window.play.workspace({kind:"create",title:"Injected"})</script><iframe src="file:///etc/passwd"></iframe><form action="https://invalid.test/"><input autofocus></form>';
    await dialog
      .getByRole("textbox", { name: "Page source", exact: true })
      .fill(hostile);
    await activate(
      dialog.getByRole("button", { name: "Add page", exact: true }),
    );
    await expect(dialog).toBeHidden();
    const cell = (title: string) =>
      notebook(page).getByRole("article", {
        name: `${title} block`,
        exact: true,
      });
    await expect(cell("Clue page")).toContainText(hostile);
    assert.equal(
      await page.evaluate(() => Object.hasOwn(globalThis, "pageExecuted")),
      false,
    );
    assert.equal(
      await page.locator("iframe,form,script:not([src])").count(),
      0,
    );
    await activate(
      cell("Clue page").getByRole("button", {
        name: "Explore Clue page",
        exact: true,
      }),
    );
    let canvas = page.getByRole("region", {
      name: "Trail canvas",
      exact: true,
    });
    await expect(canvas).toContainText("Page source · inert");
    await expect(
      canvas.getByRole("button", { name: "Run cell", exact: true }),
    ).toHaveCount(0);
    await page.keyboard.press("ControlOrMeta+Enter");
    assert.equal(
      (await current(page)).exploration.trails.filter((t) => t.pipelineId)
        .length,
      0,
    );
    await page.getByRole("tab", { name: "Stage 1", exact: true }).click();
    await activate(
      cell("Clue page").getByRole("button", { name: "Edit page", exact: true }),
    );
    dialog = page.getByRole("dialog", { name: "Edit page", exact: true });
    await dialog
      .getByRole("textbox", { name: "Page source", exact: true })
      .fill("<h1>Revised clue</h1>");
    await page.keyboard.press("ControlOrMeta+s");
    await expect(dialog).toBeHidden();
    await expect(cell("Clue page")).toContainText("Revised clue");
    await activate(
      cell("Clue page").getByRole("button", {
        name: "Duplicate Clue page",
        exact: true,
      }),
    );
    await expect(cell("Clue page (copy)")).toBeFocused();
    await expect(cell("Clue page (copy)")).toContainText(
      "Copied from Clue page · r2",
    );
    await activate(
      cell("Clue page (copy)").getByRole("button", {
        name: "Move Clue page (copy) up",
        exact: true,
      }),
    );
    await expect(cell("Clue page (copy)")).toBeFocused();
    await activate(
      cell("Clue page").getByRole("button", {
        name: "Archive Clue page",
        exact: true,
      }),
    );
    await activate(
      notebook(page).getByRole("button", {
        name: "Archived blocks (1)",
        exact: true,
      }),
    );
    const archive = page.getByRole("region", {
      name: "Archived blocks",
      exact: true,
    });
    await expect(archive).toContainText("Revised clue");
    await activate(
      archive.getByRole("button", { name: "Restore Clue page", exact: true }),
    );
    await expect(cell("Clue page")).toBeFocused();
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
    await expect(dialog).toBeHidden();
    await activate(
      cell("sample.png").getByRole("button", {
        name: "Explore sample.png",
        exact: true,
      }),
    );
    canvas = page.getByRole("region", { name: "Trail canvas", exact: true });
    await expect(canvas.locator(".operation-name")).not.toHaveText("ROT13");
    await expect(canvas).toContainText('"x":0');
    await canvas.getByRole("button", { name: "Run cell", exact: true }).focus();
    await page.keyboard.press("ControlOrMeta+Enter");
    await expect(
      canvas.getByRole("region", { name: "Cell result", exact: true }),
    ).toContainText("Base64 FFAy");
    const visibleImage = canvas.getByRole("img", {
      name: "sample.png",
      exact: true,
    });
    await expect(visibleImage).toBeInViewport();
    await expect
      .poll(
        async () => {
          const image = await visibleImage.boundingBox();
          const viewport = await canvas
            .locator(".image-viewport")
            .boundingBox();
          return Boolean(
            image &&
              viewport &&
              image.x >= viewport.x &&
              image.x + image.width <= viewport.x + viewport.width,
          );
        },
        {
          message:
            "The complete image width must be inside its own scroll viewport at 100% zoom",
        },
      )
      .toBe(true);
    const imageAccessibility = await accessibility(page);
    assert.deepEqual(imageAccessibility.violations, []);
    await capture(application, "native-image-operation");
    await page.getByRole("tab", { name: "sample.png", exact: true }).focus();
    await page.keyboard.press("Home");
    await expect(notebook(page)).toBeVisible();
    await page.keyboard.press("End");
    await expect(canvas).toBeVisible();
    const afterImage = (await current(page)).exploration;
    const imageTrail = afterImage.trails.find(
      (t) => t.source.content.kind === "image",
    );
    assert.ok(imageTrail);
    const runs = await page.evaluate(
      (t) =>
        window.play.workspace({
          kind: "list_runs",
          context: {
            workspace_id: t.workspaceId,
            exploration_id: t.explorationId,
            stage_id: t.stageId,
          },
          trail_id: t.id,
          before: null,
        }),
      imageTrail,
    );
    assert.equal(runs.runs.length, 1);
    const firstRun = runs.runs[0];
    assert.ok(firstRun);
    const detail = await page.evaluate(
      ({ t, id }) =>
        window.play.workspace({
          kind: "run_detail",
          context: {
            workspace_id: t.workspaceId,
            exploration_id: t.explorationId,
            stage_id: t.stageId,
          },
          run_id: id,
        }),
      { t: imageTrail, id: firstRun.id },
    );
    const input = detail.plan.inputs.source;
    assert.ok(input && "assetId" in input);
    assert.equal(input.assetId, imageTrail.source.id);
    assert.equal(input.semanticType, "image.samples");
    await page.getByRole("tab", { name: "Stage 1", exact: true }).click();
    await activate(
      cell("sample.png").getByRole("button", {
        name: "Explore Image caption",
        exact: true,
      }),
    );
    canvas = page.getByRole("region", { name: "Trail canvas", exact: true });
    await expect(canvas.locator(".operation-name")).toHaveText("ROT13");
    await canvas.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(canvas).toContainText("output: The key.");
    await canvas
      .getByRole("button", { name: "Save output as block", exact: true })
      .click();
    await expect(notebook(page)).toBeVisible();
    await notebook(page)
      .getByRole("button", { name: "Add annotation", exact: true })
      .click();
    dialog = page.getByRole("dialog", { name: "Add annotation", exact: true });
    await dialog
      .getByRole("textbox", { name: "Title", exact: true })
      .fill("Observation");
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Compare the selected image and caption evidence.");
    await activate(
      dialog.getByRole("button", { name: "Add annotation", exact: true }),
    );
    await expect(dialog).toBeHidden();
    const file = join(profile, "clue.html");
    await writeFile(file, hostile);
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, file);
    await activate(
      notebook(page).getByRole("button", { name: "Add file", exact: true }),
    );
    await expect(cell("clue.html")).toContainText("bytes");
    await saveNote(
      page,
      "Pages remain inert; image and caption have independent typed Runs.",
    );
    await create(page, "Exploration", "Separate notebook");
    await expect(notebook(page)).not.toContainText("Clue page");
    await explorer(page)
      .getByRole("button", { name: "Mixed clues", exact: true })
      .click();
    await expect(cell("Clue page")).toBeVisible();
    const saved = await current(page);
    const notebookAccessibility = await accessibility(page);
    assert.deepEqual(notebookAccessibility.violations, []);
    await capture(application, "native-page-notebook");
    await application.close();
    application = await launch();
    page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await expect(cell("Clue page")).toBeVisible();
    assert.deepEqual(await current(page), saved);
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

test("annotation Blocks save independently, preserve drafts and restore their Note identity", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-annotations-"));
  const launch = () =>
    electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
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
    electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
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
    electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
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
    await expect(
      canvas().getByRole("textbox", { name: "File preview", exact: true }),
    ).toHaveValue(text);
    const preview = canvas().getByRole("textbox", {
      name: "File preview",
      exact: true,
    });
    await preview.focus();
    await page.keyboard.press("ControlOrMeta+a");
    assert.equal(
      await preview.evaluate((element: HTMLTextAreaElement) =>
        element.value.slice(element.selectionStart, element.selectionEnd),
      ),
      text,
    );
    await page.keyboard.press("Tab");
    const digest = canvas().getByRole("textbox", {
      name: "File SHA-256",
      exact: true,
    });
    await expect(digest).toBeFocused();
    await expect(digest).toHaveValue(
      createHash("sha256").update(text).digest("hex"),
    );
    assert.deepEqual((await accessibility(page)).violations, []);
    await capture(application, "native-file-canvas");
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
    await expect(
      canvas().getByRole("textbox", { name: "File preview", exact: true }),
    ).toHaveValue(text);
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
    electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
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
    const enabledRetention = cell().getByRole("checkbox", {
      name: "Keep result in private history",
      exact: true,
    });
    await enabledRetention.focus();
    await page.keyboard.press("Space");
    await expect(enabledRetention).not.toBeChecked();
    await page.keyboard.press("Space");
    await expect(enabledRetention).toBeChecked();
    if (nativeWorkers) {
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
    await expect(
      canvas.getByRole("checkbox", {
        name: "Keep result in private history",
        exact: true,
      }),
    ).toBeDisabled();
    const retention = canvas.getByRole("checkbox", {
      name: "Keep result in private history",
      exact: true,
    });
    await retention.focus();
    await page.keyboard.press("Space");
    await expect(retention).toBeChecked();
    assert.deepEqual((await accessibility(page)).violations, []);
    await page.keyboard.press("ControlOrMeta+Enter");
    assert.deepEqual((await history()).runs, saved);
    if (nativeWorkers) await expect(canvas).toContainText("output: The key.");
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
    if (nativeWorkers) {
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
    electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
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
    if (nativeWorkers) {
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
    if (nativeWorkers) {
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
    if (nativeWorkers) {
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
    if (nativeWorkers)
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
    if (nativeWorkers) {
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
async function accessibility(page: Page) {
  await page.evaluate(axe.source);
  return page.evaluate(async () => {
    return window.axe.run(document, {
      runOnly: {
        type: "tag",
        values: ["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"],
      },
    });
  });
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
async function capture(
  application: ElectronApplication,
  name: string,
  resetScroll = true,
) {
  await mkdir("playwright-report", { recursive: true });
  const page = await application.firstWindow();
  if (resetScroll)
    await page.locator(".scroll-surface").evaluateAll((elements) =>
      elements.forEach((element) => {
        element.scrollTop = 0;
      }),
    );
  await page.evaluate(async () => {
    await document.fonts.ready;
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
    );
  });
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
  const editor = page.getByRole("main", {
    name: "Exploration editors",
    exact: true,
  });
  const originalWidth = (await editor.boundingBox())?.width;
  assert.ok(originalWidth);
  await page.keyboard.press("ControlOrMeta+b");
  await expect(explorer(page)).toBeHidden();
  await expect
    .poll(async () => (await editor.boundingBox())?.width ?? 0)
    .toBeGreaterThan(originalWidth + 200);
  await page.keyboard.press("ControlOrMeta+b");
  await expect(explorer(page)).toBeVisible();
  await page.keyboard.press("ControlOrMeta+Shift+p");
  const palette = page.getByRole("dialog", { name: "Command Palette" });
  await expect(palette).toBeVisible();
  assert.deepEqual((await accessibility(page)).violations, []);
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
async function pipelineReflow(application: ElectronApplication, cell = false) {
  const page = await application.firstWindow();
  const original = await application.evaluate(({ BrowserWindow }) => {
    const window = BrowserWindow.getAllWindows().find((w) => w.isVisible());
    if (!window) throw new Error("visible_window_missing");
    return {
      size: window.getBounds(),
      zoom: window.webContents.getZoomFactor(),
    };
  });
  try {
    for (const settings of [
      { width: 800, height: 540, zoom: 1 },
      { width: 1280, height: 800, zoom: 2 },
    ]) {
      await application.evaluate(({ BrowserWindow }, settings) => {
        const window = BrowserWindow.getAllWindows().find((w) => w.isVisible());
        if (!window) throw new Error("visible_window_missing");
        window.setSize(settings.width, settings.height);
        window.webContents.setZoomFactor(settings.zoom);
      }, settings);
      await expect
        .poll(() =>
          (cell
            ? page.locator(".cell-controls").first()
            : page.getByRole("dialog")
          ).evaluate((dialog) => ({
            shell: document.documentElement.scrollWidth > innerWidth,
            form: dialog.scrollWidth > dialog.clientWidth + 1,
          })),
        )
        .toEqual({ shell: false, form: false });
    }
  } finally {
    await application.evaluate(({ BrowserWindow }, original) => {
      const window = BrowserWindow.getAllWindows().find((w) => w.isVisible());
      if (!window) throw new Error("visible_window_missing");
      window.webContents.setZoomFactor(original.zoom);
      window.setSize(original.size.width, original.size.height);
    }, original);
  }
}

async function latestRun(page: Page, sourceTitle: string) {
  const state = await current(page);
  const trail = state.exploration.trails.find(
    (t) => t.source.title === sourceTitle,
  );
  assert.ok(trail);
  const context = {
    workspace_id: trail.workspaceId,
    exploration_id: trail.explorationId,
    stage_id: trail.stageId,
  };
  const detail = await page.evaluate(
    async ({ trailId, context }) => {
      const history = await window.play.workspace({
        kind: "list_runs",
        context,
        trail_id: trailId,
        before: null,
      });
      const id = history.runs[0]?.id;
      if (!id) throw new Error("run_missing");
      return window.play.workspace({
        kind: "run_detail",
        context,
        run_id: id,
      });
    },
    { trailId: trail.id, context },
  );
  return { context, detail };
}
async function frozenRun(page: Page, context: Context, id: string) {
  return page.evaluate(
    ({ context, id }) =>
      window.play.workspace({ kind: "run_detail", context, run_id: id }),
    { context, id },
  );
}

async function current(page: Page) {
  await expect(
    page.getByRole("tablist", { name: "Exploration tabs", exact: true }),
  ).toBeVisible();
  return page.evaluate(async () => {
    const list = await window.play.workspace({ kind: "list" });
    if (!list.lastWorkspaceId) throw new Error("workspace_missing");
    const workspace = await window.play.workspace({
      kind: "view",
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
  const renderer = await application.evaluate(({ app, BrowserWindow }) => ({
    disabled: app.commandLine.hasSwitch("no-sandbox"),
    pid: BrowserWindow.getAllWindows()[0]?.webContents.getOSProcessId(),
  }));
  assert.equal(renderer.disabled, false);
  assert.ok(renderer.pid);
  if (process.platform === "linux") {
    const status = await readFile(`/proc/${renderer.pid}/status`, "utf8");
    assert.match(status, /Seccomp:\s+2/);
    assert.match(status, /NoNewPrivs:\s+1/);
  }
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
        () =>
          window.play.importChallenge({
            workspace_id: context.workspace_id,
            key: crypto.randomUUID(),
          }),
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
    [true, true, true, true, true],
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
    electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
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
    await expect(page.locator(".operation-name")).toHaveText("ROT13");
    await imageBlock
      .getByRole("button", { name: "Run cell", exact: true })
      .click();
    const result = nativeWorkers
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
      2,
      "text and image Trails reuse their own compatible Workspace pipelines",
    );
    assert.equal(
      second.workspace.pipelines.filter(
        (p) => p.outputOperation === "cipher.rot13",
      ).length,
      1,
    );
    assert.equal(
      second.workspace.pipelines.filter(
        (p) => p.outputOperation === "image.pixels.read",
      ).length,
      1,
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
            trail_revision: 1,
            target_node_id: crypto.randomUUID(),
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
    await expect(
      canvas.getByRole("button", { name: "Run cell" }),
    ).toBeEnabled();
    await expect(canvas.locator(".operation-name")).not.toHaveText("ROT13");
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
    await expect(restoredResult).toContainText("Out [2]");
    await expect(restoredResult).toContainText("The key is in the pixels.");
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
    electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
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
    if (nativeWorkers) {
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
    if (nativeWorkers) {
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
    // Filling the renderer is not an acknowledgement from Electron main.
    // Observe the real dirty notification on every OS; isDocumentEdited is
    // macOS-only. Keep the native call, rather than forging a dirty IPC message.
    const dirtyState = await application.evaluateHandle(({ BrowserWindow }) => {
      const window = BrowserWindow.getAllWindows()[0];
      if (!window) throw new Error("window_missing");
      const state = { dirty: false };
      const setEdited = window.setDocumentEdited.bind(window);
      window.setDocumentEdited = (dirty) => {
        setEdited(dirty);
        state.dirty = dirty;
      };
      return state;
    });
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Unsaved source draft.");
    await expect
      .poll(() => dirtyState.evaluate((state) => state.dirty))
      .toBe(true);
    await dirtyState.dispose();
    if (nativeWorkers) {
      if (process.platform === "darwin")
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
    if (nativeWorkers) {
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
    if (nativeWorkers) {
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
    if (nativeWorkers) {
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
    electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
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
    if (nativeWorkers) {
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
    if (nativeWorkers) {
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
    if (nativeWorkers)
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
    chromiumSandbox: true,
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

test("native notebook queues another cell without disabling unrelated Trails", {
  skip: !nativeWorkers,
  timeout: 60000,
}, async () => {
  // Stage a task-owned application with the platform supervision helper. The
  // production app/worker, resource ceilings and integrity checks stay intact.
  const temporary = await mkdtemp(join(tmpdir(), "play-queue-ui-"));
  const staged = join(temporary, "frontend");
  const front = fileURLToPath(new URL("../", import.meta.url));
  await cp(join(front, "desktop-dist"), join(staged, "desktop-dist"), {
    recursive: true,
  });
  await cp(join(front, "../desktop"), join(temporary, "desktop"), {
    recursive: true,
  });
  await mkdir(join(staged, "desktop-native"));
  for (const name of ["desktop-renderer", "node_modules"])
    await symlink(join(front, name), join(staged, name));
  for (const name of process.platform === "darwin"
    ? ["play-engine", "cube", "Terminal.app"]
    : ["play-engine", "cube", "play-terminal"])
    await symlink(
      join(front, "desktop-native", name),
      join(staged, "desktop-native", name),
    );
  const files = nativeFiles();
  assert.ok(files.worker);
  if (process.platform === "darwin") {
    await cp(
      join(front, "desktop-native/SupervisionProbe.app"),
      join(staged, "desktop-native/Worker.app"),
      { recursive: true },
    );
  } else {
    await cp(
      join(front, "desktop-native/native-supervision-probe"),
      join(staged, "desktop-native", files.worker),
    );
  }
  const manifest = JSON.parse(
    await readFile(join(front, "desktop-native/integrity.json"), "utf8"),
  );
  manifest.worker = createHash("sha256")
    .update(await readFile(join(staged, "desktop-native", files.worker)))
    .digest("hex");
  await writeFile(
    join(staged, "desktop-native/integrity.json"),
    JSON.stringify(manifest),
  );
  await writeFile(
    join(staged, "package.json"),
    JSON.stringify({ type: "module" }),
  );
  const application = await electron.launch({
    chromiumSandbox: true,
    args: [
      join(staged, "desktop-dist/main.js"),
      `--user-data-dir=${join(staged, "profile")}`,
    ],
  });
  try {
    const page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Concurrent clues");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Independent paths");
    for (const [title, text] of [
      ["First clue", "Uryyb"],
      ["Second clue", "Jbeyq"],
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
    const first = notebook(page).getByRole("article", {
      name: "First clue block",
      exact: true,
    });
    const second = notebook(page).getByRole("article", {
      name: "Second clue block",
      exact: true,
    });
    await first.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(first.getByRole("status")).toHaveText("Running…");
    await expect(
      second.getByRole("button", { name: "Run cell", exact: true }),
    ).toBeEnabled();
    await second.getByRole("button", { name: "Run cell", exact: true }).click();
    await expect(second.getByRole("status")).toHaveText("Queued…");
    await expect(first.getByRole("status")).toHaveText("Running…");
    await capture(application, "native-run-queue", false);
    await second.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(second.getByRole("status")).toHaveText(
      "Cancelled: user_cancelled",
    );
    await expect(first.getByRole("status")).toHaveText("Running…");
    await first.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(first.getByRole("status")).toHaveText(
      "Cancelled: user_cancelled",
    );
    await expect(
      second.getByRole("button", { name: "Run cell", exact: true }),
    ).toBeEnabled();
    assert.equal((await accessibility(page)).violations.length, 0);
  } finally {
    await application.close();
    await rm(temporary, { recursive: true, force: true });
  }
});

test("canonical Challenge import creates editable grouped content and rejects partial imports", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-challenge-ui-"));
  const original = await readFile(
    new URL("../../docs/assets/challenge-example.json", import.meta.url),
  );
  const source = join(profile, "challenge.json");
  await writeFile(source, original);
  const launch = () =>
    electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
  let application = await launch();
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Local Challenges");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: "Import Challenge", exact: true }),
    ).toBeVisible();
    await application.evaluate(({ dialog }) => {
      dialog.showOpenDialog = async () => ({ canceled: true, filePaths: [] });
    });
    await page
      .getByRole("button", { name: "Import Challenge", exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: "Import Challenge", exact: true }),
    ).toBeEnabled();
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, source);
    await page
      .getByRole("button", { name: "Import Challenge", exact: true })
      .click();
    await expect(
      notebook(page).getByRole("heading", {
        name: "Read the image",
        exact: true,
      }),
    ).toBeVisible();
    const image = () =>
      notebook(page).getByRole("article", {
        name: "Spiral image block",
        exact: true,
      });
    await expect(
      image().getByRole("img", { name: "Spiral image", exact: true }),
    ).toBeVisible();
    await expect(image().getByRole("figure")).toContainText(
      "Gur xrl vf va gur cvkryf.",
    );
    await expect(
      notebook(page).getByRole("article", {
        name: "Clue page block",
        exact: true,
      }),
    ).toContainText("<script>window.packageExecuted = true</script>");
    assert.equal(
      await page.evaluate(
        () => (window as unknown as Record<string, unknown>).packageExecuted,
      ),
      undefined,
    );
    await page
      .getByText("Imported Challenge · revision 1", { exact: true })
      .click();
    await expect(notebook(page)).toContainText(
      JSON.parse(original.toString()).id,
    );
    await expect(image()).not.toContainText("Completed");
    if (nativeWorkers) {
      await image()
        .getByRole("button", { name: "Run cell", exact: true })
        .click();
      await expect(image()).toContainText("output: The key is in the pixels.");
    }
    await saveNote(page, "My local reasoning survives package import.");
    assert.deepEqual((await accessibility(page)).violations, []);
    await page
      .getByText("Imported Challenge · revision 1", { exact: true })
      .click();
    await capture(application, "challenge-import");
    await application.evaluate(({ BrowserWindow }) => {
      const window = BrowserWindow.getAllWindows()[0];
      if (!window) throw new Error("missing_window");
      window.setSize(800, 700);
      window.webContents.setZoomFactor(2);
    });
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
      true,
    );
    await expect(
      page.getByText("Imported Challenge · revision 1", { exact: true }),
    ).toBeVisible();
    await application.evaluate(({ BrowserWindow }) => {
      const window = BrowserWindow.getAllWindows()[0];
      if (!window) throw new Error("missing_window");
      window.setSize(1280, 900);
      window.webContents.setZoomFactor(1);
    });
    const identity = JSON.parse(original.toString());
    identity.stages[0].blocks[3].content.sha256 = "invalid";
    await writeFile(source, JSON.stringify(identity));
    await page
      .getByRole("button", { name: "Import Challenge", exact: true })
      .click();
    await expect(page.getByRole("alert")).toContainText(
      "does not match its recorded checksum",
    );
    await expect(
      page
        .getByRole("navigation", { name: "Explorations", exact: true })
        .getByRole("button", { name: "Signals in a spiral", exact: true }),
    ).toHaveCount(1);
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "My local reasoning survives package import.",
    );
    await page
      .getByRole("button", { name: "Dismiss error", exact: true })
      .click();
    const moved = join(profile, "moved.json");
    await writeFile(moved, original);
    await application.evaluate(({ dialog }, path) => {
      dialog.showOpenDialog = async () => ({
        canceled: false,
        filePaths: [path],
      });
    }, moved);
    await page
      .getByRole("button", { name: "Import Challenge", exact: true })
      .click();
    await expect(
      page
        .getByRole("navigation", { name: "Explorations", exact: true })
        .getByRole("button", { name: "Signals in a spiral", exact: true }),
    ).toHaveCount(2);
    await expect(page.getByLabel("Stage note", { exact: true })).toHaveValue(
      "",
    );
    await page
      .getByText("Imported Challenge · revision 1", { exact: true })
      .click();
    await expect(notebook(page)).toContainText(identity.id);
    assert.deepEqual(await readFile(moved), original);
    await application.close();
    application = await launch();
    page = await application.firstWindow();
    await expect(
      notebook(page).getByRole("heading", {
        name: "Read the image",
        exact: true,
      }),
    ).toBeVisible();
    await expect(
      page
        .getByRole("navigation", { name: "Explorations", exact: true })
        .getByRole("button", { name: "Signals in a spiral", exact: true }),
    ).toHaveCount(2);
    await page
      .getByRole("button", { name: "02 · Compare the clues", exact: true })
      .click();
    await expect(notebook(page)).toContainText(
      "What do the image and caption have in common?",
    );
  } finally {
    await application.close();
    await rm(profile, { recursive: true, force: true });
  }
});

test("Stage candidates and Solutions preserve chosen evidence, handle conflicts and reopen independently", {
  timeout: 90000,
}, async () => {
  const profile = await mkdtemp(join(tmpdir(), "play-candidates-"));
  const launch = () =>
    electron.launch({
      chromiumSandbox: true,
      args: [main, `--user-data-dir=${profile}`],
    });
  let application = await launch();
  try {
    let page = await application.firstWindow();
    page.setDefaultTimeout(5000);
    await page
      .getByRole("textbox", { name: "Workspace name", exact: true })
      .fill("Selected answers");
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
    await create(page, "Exploration", "Hidden message");
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
    const panel = () =>
      notebook(page).getByRole("region", {
        name: "Stage candidate",
        exact: true,
      });
    const back = async () => {
      await page.keyboard.press("ControlOrMeta+Shift+p");
      await page
        .getByRole("textbox", { name: "Command", exact: true })
        .fill("Stage notebook");
      await page.keyboard.press("Enter");
    };
    await cell()
      .getByRole("button", { name: "Explore Clue", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Choose Clue as candidate", exact: true })
      .click();
    await expect(
      page.getByRole("button", {
        name: "Selected Clue as candidate",
        exact: true,
      }),
    ).toBeVisible();
    await back();
    await expect(panel()).toContainText("Gur xrl.");
    await cell()
      .getByRole("button", { name: "Edit text", exact: true })
      .click();
    dialog = page.getByRole("dialog", { name: "Edit text", exact: true });
    await dialog
      .getByRole("textbox", { name: "Text", exact: true })
      .fill("Svefg pyhr.");
    await dialog
      .getByRole("button", { name: "Save changes", exact: true })
      .click();
    await expect(dialog).toBeHidden();
    await expect(panel()).toContainText("Gur xrl.");
    await expect(panel()).toContainText("Asset r1");
    if (nativeWorkers) {
      await cell()
        .getByRole("button", { name: "Run cell", exact: true })
        .click();
      await expect(cell()).toContainText("output: First clue.");
      const snapshot = await current(page);
      await page.evaluate(async ({ workspace, exploration }) => {
        const asset = exploration.trails[0]?.source;
        if (!asset) throw new Error("missing_asset");
        await window.play.workspace({
          kind: "select_candidate",
          context: {
            workspace_id: workspace.id,
            exploration_id: exploration.id,
            stage_id: exploration.stage.id,
          },
          key: crypto.randomUUID(),
          expected_revision: 1,
          selection: { kind: "asset", asset_id: asset.id, revision: 1 },
        });
      }, snapshot);
      await cell()
        .getByRole("button", {
          name: "Choose output as candidate",
          exact: true,
        })
        .click();
      await expect(panel().getByRole("alert")).toContainText(
        "Stage candidate changed",
      );
      await cell()
        .getByRole("button", {
          name: "Choose output as candidate",
          exact: true,
        })
        .click();
      await expect(panel()).toContainText("First clue.");
      await expect(panel()).toContainText("Private · Unverified");
      await panel()
        .getByLabel("Candidate revision", { exact: true })
        .selectOption("1");
      await expect(panel()).toContainText("Gur xrl.");
      await panel()
        .getByLabel("Candidate revision", { exact: true })
        .selectOption("3");
      await expect(panel()).toContainText("First clue.");
      await panel()
        .getByRole("button", { name: "Open source Run", exact: true })
        .click();
      await expect(
        page.getByRole("region", { name: "Trail canvas" }),
      ).toContainText("output: First clue.");
      await back();
    }
    const solutions = () =>
      notebook(page).getByRole("region", {
        name: "Stage solutions",
        exact: true,
      });
    await solutions()
      .getByRole("heading", { name: "Solutions", exact: true })
      .click();
    await solutions()
      .getByRole("button", { name: /New solution from Candidate/ })
      .click();
    await solutions()
      .getByLabel("Solution title", { exact: true })
      .fill("The first clue");
    await solutions()
      .getByLabel("Solution explanation", { exact: true })
      .fill("ROT13 reveals the selected clue. This is my local explanation.");
    await solutions()
      .getByRole("button", { name: "Save solution", exact: true })
      .click();
    const saved = () =>
      solutions().getByRole("article", { name: "Saved solution", exact: true });
    await expect(saved()).toContainText("This is my local explanation.");
    await expect(saved()).toContainText(
      nativeWorkers ? "First clue." : "Gur xrl.",
    );
    if (nativeWorkers) await expect(saved()).toContainText("Contributing path");
    await solutions()
      .getByRole("button", { name: "Revise explanation", exact: true })
      .click();
    await solutions()
      .getByLabel("Solution explanation", { exact: true })
      .fill("A revised explanation that preserves the same evidence.");
    const owner = await current(page);
    await page.evaluate(async ({ workspace, exploration }) => {
      const context = {
        workspace_id: workspace.id,
        exploration_id: exploration.id,
        stage_id: exploration.stage.id,
      };
      const list = await window.play.workspace({
        kind: "list_solutions",
        context,
      });
      const item = list.solutions[0];
      if (!item) throw new Error("missing_solution");
      const detail = await window.play.workspace({
        kind: "get_solution",
        context,
        solution_id: item.id,
        revision: null,
      });
      await window.play.workspace({
        kind: "save_solution",
        context,
        key: crypto.randomUUID(),
        solution_id: item.id,
        expected_revision: 1,
        candidate_revision: detail.snapshot.candidate.revision,
        title: item.title,
        explanation: "An independently saved explanation.",
      });
    }, owner);
    await solutions()
      .getByRole("button", { name: "Save solution", exact: true })
      .click();
    await expect(solutions().getByRole("alert")).toContainText(
      "Your draft is preserved",
    );
    await expect(
      solutions().getByLabel("Solution explanation", { exact: true }),
    ).toHaveValue("A revised explanation that preserves the same evidence.");
    await solutions()
      .getByRole("button", { name: "Save solution", exact: true })
      .click();
    await expect(saved()).toContainText("A revised explanation");
    await saved()
      .getByLabel("Solution revision", { exact: true })
      .selectOption("1");
    await expect(saved()).toContainText("This is my local explanation.");
    await saved()
      .getByLabel("Solution revision", { exact: true })
      .selectOption("3");
    await solutions().evaluate((e) => e.scrollIntoView({ block: "start" }));
    await capture(application, "desktop-stage-solution", false);
    assert.deepEqual((await accessibility(page)).violations, []);
    await capture(application, "desktop-stage-candidate");
    for (const zoom of [1, 2]) {
      await application.evaluate(({ BrowserWindow }, zoom) => {
        const win = BrowserWindow.getAllWindows().find((w) => w.isVisible());
        if (!win) throw new Error("missing_window");
        win.setSize(860, 640);
        win.webContents.setZoomFactor(zoom);
      }, zoom);
      await expect(panel()).toBeVisible();
      await expect
        .poll(() =>
          solutions().evaluate((e) => e.scrollWidth <= e.clientWidth + 1),
        )
        .toBe(true);
      assert.ok(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth + 1,
        ),
      );
      await expect
        .poll(() =>
          panel().evaluate((e) => ({
            width: e.clientWidth,
            overflow: e.scrollWidth > e.clientWidth + 1,
          })),
        )
        .toMatchObject({ overflow: false });
    }
    await application.evaluate(({ BrowserWindow }) => {
      const win = BrowserWindow.getAllWindows().find((w) => w.isVisible());
      win?.webContents.setZoomFactor(1);
      win?.setSize(1280, 800);
    });
    await create(page, "Exploration", "Independent answer");
    await expect(panel()).toContainText("Choose an asset");
    await expect(solutions()).toContainText("Explain a selected candidate");
    await explorer(page)
      .getByRole("button", { name: "Hidden message", exact: true })
      .click();
    await expect(panel()).toContainText(
      nativeWorkers ? "First clue." : "Gur xrl.",
    );
    await application.close();
    application = await launch();
    page = await application.firstWindow();
    await expect(panel()).toContainText(
      nativeWorkers ? "First clue." : "Gur xrl.",
    );
    await panel()
      .getByLabel("Candidate revision", { exact: true })
      .selectOption("1");
    await expect(panel()).toContainText("Gur xrl.");
    await solutions()
      .getByRole("heading", { name: "Solutions", exact: true })
      .click();
    await expect(saved()).toContainText("A revised explanation");
    await saved()
      .getByLabel("Solution revision", { exact: true })
      .selectOption("1");
    await expect(saved()).toContainText("This is my local explanation.");
  } finally {
    await application.close();
    await rm(profile, { recursive: true, force: true });
  }
});
