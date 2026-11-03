//! Real previous/current macOS apps and manual replacement; no automatic updater claim.
import assert from "node:assert/strict";
import { cp, mkdtemp, readFile, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";
import {
  type ElectronApplication,
  _electron as electron,
  expect,
} from "@playwright/test";
import type {} from "./bridge.ts";
import { newer } from "./update-metadata.ts";

async function proof(app: string) {
  return JSON.parse(
    await readFile(
      join(app, "Contents/Resources/app/package-proof.json"),
      "utf8",
    ),
  );
}
async function launch(app: string, profile: string) {
  const application = await electron.launch({
    executablePath: join(app, "Contents/MacOS/cube-play"),
    chromiumSandbox: true,
    args: [`--user-data-dir=${profile}`],
    env: { PATH: "/usr/bin:/bin" },
  });
  try {
    const page = await application.firstWindow();
    await page.waitForFunction(
      () => typeof window.play?.workspace === "function",
    );
    await expect(
      page.getByRole("button", { name: "Workspaces", exact: true }),
    ).toBeEnabled();
    return application;
  } catch (error) {
    console.error("install_startup", {
      exitCode: application.process().exitCode,
      signalCode: application.process().signalCode,
    });
    await application.close().catch(() => {});
    throw error;
  }
}

test("real Mac previews preserve evidence through install, replacement, manual rollback, removal and reinstallation", {
  skip: process.platform !== "darwin",
  timeout: 180000,
}, async () => {
  if (!process.env.PLAY_PREVIOUS_MAC_APP || !process.env.PLAY_CURRENT_MAC_APP)
    throw new Error(
      "Set PLAY_PREVIOUS_MAC_APP and PLAY_CURRENT_MAC_APP to real clean packaged applications.",
    );
  const previous = resolve(process.env.PLAY_PREVIOUS_MAC_APP),
    current = resolve(process.env.PLAY_CURRENT_MAC_APP);
  const beforeProof = await proof(previous),
    afterProof = await proof(current);
  assert.equal(beforeProof.sourceDirty, false);
  assert.equal(afterProof.sourceDirty, false);
  assert.ok(newer(afterProof.appVersion, beforeProof.appVersion));
  const root = await mkdtemp(join(tmpdir(), "play-real-install-"));
  const installed = join(root, "Cube Play.app"),
    incoming = join(root, "incoming.app"),
    backup = join(root, "previous.app"),
    profile = join(root, "profile");
  let application: ElectronApplication | undefined;
  try {
    await cp(previous, installed, { recursive: true, verbatimSymlinks: true });
    application = await launch(installed, profile);
    let page = await application.firstWindow();
    const saved = await page.evaluate(async () => {
      const workspace = await window.play.workspace({
        kind: "create",
        key: crypto.randomUUID(),
        title: "Installation evidence",
      });
      const e = await window.play.workspace({
        kind: "create_exploration",
        workspace_id: workspace.id,
        key: crypto.randomUUID(),
        title: "First investigation",
      });
      const context = {
        workspace_id: workspace.id,
        exploration_id: e.explorationId,
        stage_id: e.stageId,
      };
      await window.play.workspace({
        kind: "add_text",
        context,
        key: crypto.randomUUID(),
        title: "Original clue",
        text: "Gur xrl.",
      });
      const view = await window.play.workspace({
        kind: "open_exploration",
        workspace_id: workspace.id,
        exploration_id: e.explorationId,
        stage_id: e.stageId,
      });
      const trail = view.trails[0];
      if (!trail?.pipeline) throw new Error("pipeline_missing");
      let run = await window.play.startRun({
        context,
        key: crypto.randomUUID(),
        trail_id: trail.id,
        trail_revision: trail.revision,
        target_node_id: trail.pipeline.outputNodeId,
        pipeline_id: trail.pipeline.id,
        pipeline_revision: trail.pipeline.revision,
        source_asset_id: trail.source.id,
        source_revision: trail.source.revision,
        operation: "cipher.rot13",
        operation_version: 1,
        retain_output: true,
      });
      for (
        let i = 0;
        i < 100 && ["queued", "running"].includes(run.status);
        i++
      ) {
        await new Promise((done) => setTimeout(done, 25));
        run = await window.play.workspace({
          kind: "run_detail",
          context,
          run_id: run.id,
        });
      }
      if (
        run.status !== "completed" ||
        run.result?.outputs?.output !== "The key."
      )
        throw new Error("real_run_failed");
      const attempt = run.attempts[0];
      if (!attempt) throw new Error("attempt_missing");
      await window.play.workspace({
        kind: "select_candidate",
        context,
        key: crypto.randomUUID(),
        expected_revision: 0,
        selection: {
          kind: "run_output",
          run_id: run.id,
          attempt_id: attempt.id,
          port: "output",
        },
      });
      const solution = crypto.randomUUID();
      await window.play.workspace({
        kind: "save_solution",
        context,
        key: crypto.randomUUID(),
        solution_id: solution,
        expected_revision: 0,
        candidate_revision: 1,
        notes: [],
        title: "Chosen result",
        explanation: "ROT13 decodes the selected original clue.",
      });
      const other = await window.play.workspace({
        kind: "create_exploration",
        workspace_id: workspace.id,
        key: crypto.randomUUID(),
        title: "Independent investigation",
      });
      const foreign = {
        workspace_id: workspace.id,
        exploration_id: other.explorationId,
        stage_id: other.stageId,
      };
      await window.play.workspace({
        kind: "add_text",
        context: foreign,
        key: crypto.randomUUID(),
        title: "Separate clue",
        text: "Never merge this context.",
      });
      return { context, foreign, run: run.id, solution };
    });
    const capture = async () =>
      page.evaluate(async (saved) => {
        await window.play.workspace({
          kind: "open",
          workspace_id: saved.context.workspace_id,
        });
        return {
          first: await window.play.workspace({
            kind: "open_exploration",
            ...saved.context,
          }),
          second: await window.play.workspace({
            kind: "open_exploration",
            ...saved.foreign,
          }),
          run: await window.play.workspace({
            kind: "run_detail",
            context: saved.context,
            run_id: saved.run,
          }),
          solution: await window.play.workspace({
            kind: "get_solution",
            context: saved.context,
            solution_id: saved.solution,
            revision: 1,
          }),
        };
      }, saved);
    const original = await capture();
    await application.close();
    application = undefined;
    // Abandoned preparation leaves the installed app intact.
    await cp(current, incoming, { recursive: true, verbatimSymlinks: true });
    await rm(incoming, { recursive: true });
    assert.deepEqual(await proof(installed), beforeProof);
    await cp(current, incoming, { recursive: true, verbatimSymlinks: true });
    await rename(installed, backup);
    await rename(incoming, installed);
    application = await launch(installed, profile);
    page = await application.firstWindow();
    assert.deepEqual(await capture(), original);
    await application.close();
    application = undefined;
    await rm(installed, { recursive: true });
    await rename(backup, installed);
    application = await launch(installed, profile);
    page = await application.firstWindow();
    assert.deepEqual(await capture(), original);
    await application.close();
    application = undefined;
    // Removal deletes only the owned app fixture. The private profile is re-used.
    await rm(installed, { recursive: true });
    await cp(current, installed, { recursive: true, verbatimSymlinks: true });
    application = await launch(installed, profile);
    page = await application.firstWindow();
    assert.deepEqual(await capture(), original);
    if (process.env.PLAY_INSTALL_PROOF)
      await writeFile(
        process.env.PLAY_INSTALL_PROOF,
        JSON.stringify(
          {
            previous: beforeProof,
            current: afterProof,
            checks: [
              "real_core_run",
              "independent_explorations",
              "solution_snapshot",
              "abandoned_preparation",
              "manual_replacement",
              "manual_rollback",
              "removal_reinstallation",
            ],
            automaticUpdater: false,
          },
          null,
          2,
        ),
      );
  } finally {
    await application?.close();
    await rm(root, { recursive: true, force: true });
  }
});
