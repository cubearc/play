import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test, { after } from "node:test";
import { fileURLToPath } from "node:url";
import { crc32, deflateSync } from "node:zlib";
import { contract, type Operation } from "./contract.js";
import { Engine } from "./engine.js";
import {
  type Context,
  contextOf,
  type ExplorationView,
  type PipelineDefinition,
  type RunDetail,
} from "./workspace-contract.js";

interface Fixture {
  operation: string;
  operation_version: number;
  contract_sha256: string;
  inputs: Record<string, unknown>;
  parameters: Record<string, unknown>;
  outputs: Record<string, unknown>;
}
// Substitute only canonical defaults with the host's effective defaults. This
// keeps interpreter ceilings bounded; every explicit fixture choice is retained.
function hostParameters(
  operation: Operation,
  fixture: Fixture,
): Record<string, unknown> {
  const schema = operation.contract.parameters as {
    properties: Record<string, { default?: unknown }>;
  };
  return {
    ...operation.parameters,
    ...Object.fromEntries(
      Object.entries(fixture.parameters).map(([name, value]) => [
        name,
        Object.hasOwn(operation.parameters, name) &&
        Object.hasOwn(schema.properties[name] ?? {}, "default") &&
        JSON.stringify(schema.properties[name]?.default) ===
          JSON.stringify(value)
          ? operation.parameters[name]
          : value,
      ]),
    ),
  };
}

interface Raster {
  width: number;
  height: number;
  color_model: string;
  samples: { base64: string };
  palette: number[][];
  transparency: { base64: string };
}
// Lossless PNG test containers for fixed Core vectors, not product image processing.
function fixturePng(value: unknown): Buffer {
  const image = value as Raster;
  const model: Record<string, [number, number]> = {
    gray: [0, 1],
    rgb: [2, 3],
    indexed: [3, 1],
    gray_alpha: [4, 2],
    rgba: [6, 4],
  };
  const layout = model[image.color_model];
  assert.ok(layout);
  const chunk = (name: string, data: Buffer) => {
    const kind = Buffer.from(name);
    const bytes = Buffer.alloc(data.length + 12);
    bytes.writeUInt32BE(data.length);
    kind.copy(bytes, 4);
    data.copy(bytes, 8);
    bytes.writeUInt32BE(crc32(Buffer.concat([kind, data])), data.length + 8);
    return bytes;
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(image.width);
  header.writeUInt32BE(image.height, 4);
  header[8] = 8;
  header[9] = layout[0];
  const samples = Buffer.from(image.samples.base64, "base64"),
    stride = image.width * layout[1];
  assert.equal(samples.length, stride * image.height);
  const rows = Buffer.alloc((stride + 1) * image.height);
  for (let row = 0; row < image.height; row++)
    samples.copy(
      rows,
      row * (stride + 1) + 1,
      row * stride,
      (row + 1) * stride,
    );
  return Buffer.concat([
    Buffer.from("89504e470d0a1a0a", "hex"),
    chunk("IHDR", header),
    ...(image.palette.length
      ? [chunk("PLTE", Buffer.from(image.palette.flat()))]
      : []),
    ...(image.transparency.base64
      ? [chunk("tRNS", Buffer.from(image.transparency.base64, "base64"))]
      : []),
    chunk("IDAT", deflateSync(rows)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

const binary = fileURLToPath(
  new URL(
    process.platform === "win32"
      ? "../desktop-native/play-engine.exe"
      : "../desktop-native/play-engine",
    import.meta.url,
  ),
);
const worker =
  process.platform === "darwin"
    ? fileURLToPath(
        new URL(
          "../desktop-native/Worker.app/Contents/MacOS/play-engine",
          import.meta.url,
        ),
      )
    : process.platform === "linux"
      ? binary
      : undefined;

const registryEngine = await Engine.connect(binary);
const discovered = await registryEngine
  .catalog()
  .finally(() => registryEngine.close());
const available = discovered.filter(
  (entry) => entry.availability.state === "available",
);
assert.ok(available.length > 0);
assert.equal(
  new Set(discovered.map((entry) => entry.operation)).size,
  discovered.length,
);
const qualified: unknown[] = [];
// Bound each independent native matrix unit by the original per-test deadline.
// A larger catalog adds units; it never extends a worker or test deadline.
for (
  let batch = 0;
  batch * contract.workspace_limit < available.length;
  batch++
) {
  const catalog = available.slice(
    batch * contract.workspace_limit,
    (batch + 1) * contract.workspace_limit,
  );
  test(`Core native known answers and restart evidence, batch ${batch + 1}`, {
    timeout: 60000,
  }, async (t) => {
    const root = await mkdtemp(join(tmpdir(), "play-native-registry-matrix-"));
    let engine = await Engine.connect(binary, worker, join(root, "private"));
    let phase = "load fixtures";
    let currentOperation = "catalog";
    try {
      const vectors = JSON.parse(
        await readFile(
          new URL(
            "../../../cube/crates/cube-core/src/operations/contracts/conformance.json",
            import.meta.url,
          ),
          "utf8",
        ),
      ) as Fixture[];
      const report: unknown[] = [];
      let caseRoot = "";
      const retained: {
        workspaceId: string;
        context: Context;
        run: RunDetail;
      }[] = [];
      const verifyRestart = async () => {
        if (!retained.length) return;
        await engine.close();
        engine = await Engine.connect(binary, worker, caseRoot);
        for (const { workspaceId, context, run } of retained) {
          await engine.workspace({ kind: "open", workspace_id: workspaceId });
          assert.deepEqual(
            await engine.workspace({
              kind: "run_detail",
              context,
              run_id: run.id,
            }),
            run,
          );
        }
        retained.length = 0;
      };
      for (const [index, operation] of catalog.entries()) {
        currentOperation = operation.operation;
        phase = "create Workspace and Exploration";
        t.diagnostic(operation.operation);
        if (index % contract.workspace_limit === 0) {
          await verifyRestart();
          await engine.close();
          caseRoot = join(root, randomUUID());
          engine = await Engine.connect(binary, worker, caseRoot);
        }
        const workspace = (await engine.workspace({
          kind: "create",
          key: randomUUID(),
          title: "Core registry matrix",
        })) as { id: string };
        const created = (await engine.workspace({
          kind: "create_exploration",
          workspace_id: workspace.id,
          key: randomUUID(),
          title: "Registered known answers",
        })) as { explorationId: string; stageId: string };
        const open = {
          kind: "open_exploration" as const,
          workspace_id: workspace.id,
          exploration_id: created.explorationId,
          stage_id: created.stageId,
        };
        const context = contextOf(
          (await engine.workspace(open)) as ExplorationView,
        );
        await engine.workspace({
          kind: "add_text",
          context,
          key: randomUUID(),
          title: "Fixture anchor",
          text: "Unused input",
        });

        const fixture = vectors.find(
          (f) => f.operation === operation.operation,
        );
        assert.ok(fixture, `missing Core vector: ${operation.operation}`);
        assert.equal(fixture.operation_version, operation.operation_version);
        phase = "materialize exact fixture Assets";
        const assetPorts = operation.contract.inputs.filter(
          (p) =>
            ["image_samples", "bytes", "byte_array", "json"].includes(
              operation.execution.inputs[p.name] ?? "",
            ) && Object.hasOwn(fixture.inputs, p.name),
        );
        const assets = new Map<string, ExplorationView["trails"][number]>();
        for (const port of assetPorts) {
          const before = (await engine.workspace(open)) as ExplorationView;
          if (operation.execution.inputs[port.name] === "image_samples") {
            await engine.workspace({
              kind: "add_image",
              context,
              key: randomUUID(),
              title: `Fixture ${operation.name}`,
              caption: "",
              png: fixturePng(fixture.inputs[port.name]).toString("base64"),
            });
          } else if (operation.execution.inputs[port.name] === "json") {
            await engine.workspace({
              kind: "add_structured",
              context,
              key: randomUUID(),
              title: `Fixture ${operation.name}`,
              semantic_type: port.semantic_type,
              value: fixture.inputs[
                port.name
              ] as import("./workspace-contract.js").JsonValue,
            });
          } else {
            const value: { base64: string } =
              operation.execution.inputs[port.name] === "byte_array"
                ? {
                    base64: Buffer.from(
                      fixture.inputs[port.name] as number[],
                    ).toString("base64"),
                  }
                : (fixture.inputs[port.name] as { base64: string });
            assert.equal(typeof value.base64, "string");
            await engine.workspace({
              kind: "add_file",
              context,
              key: randomUUID(),
              title: `Fixture ${operation.name}`,
              base64: value.base64,
            });
          }
          const after = (await engine.workspace(open)) as ExplorationView;
          const created = after.trails.find(
            (t) => !before.trails.some((old) => old.id === t.id),
          );
          assert.ok(created);
          assets.set(port.name, created);
        }
        const view = (await engine.workspace(open)) as ExplorationView;
        const trail =
          assets.values().next().value ??
          view.trails.find((t) => t.source.content.kind === "text");
        assert.ok(trail);
        const assetSlots = new Map(
          assetPorts.map((p, i) => [p.name, i === 0 ? "source" : `asset_${i}`]),
        );
        const bindings = Object.fromEntries(
          assetPorts.slice(1).map((p) => {
            const image = assets.get(p.name);
            assert.ok(image);
            return [
              assetSlots.get(p.name) ?? "",
              { asset_id: image.source.id, revision: image.source.revision },
            ];
          }),
        );
        const node = randomUUID();
        const parameters = hostParameters(operation, fixture);
        const definition: PipelineDefinition = {
          formatVersion: 4,
          inputs: assetPorts.length
            ? assetPorts.map((p) => ({
                name: assetSlots.get(p.name) ?? "",
                semanticType: p.semantic_type,
              }))
            : [{ name: "source", semanticType: "text" }],
          nodes: [
            {
              id: node,
              operation: operation.operation,
              operationVersion: operation.operation_version,
              parameters,
              ...(operation.requires_private_retention
                ? { retainPrivate: true }
                : {}),
              position: { x: 80, y: 80 },
              inputs: Object.fromEntries(
                Object.entries(fixture.inputs).map(([name, value]) => [
                  name,
                  assetSlots.has(name)
                    ? {
                        kind: "input" as const,
                        name: assetSlots.get(name) ?? "",
                      }
                    : { kind: "literal" as const, value },
                ]),
              ),
            },
          ],
          output: node,
        };
        if (operation.requires_private_retention) {
          const unconsented = structuredClone(definition);
          delete unconsented.nodes[0]?.retainPrivate;
          await assert.rejects(
            engine.workspace({
              kind: "create_pipeline",
              workspace_id: workspace.id,
              key: randomUUID(),
              title: operation.name,
              definition: unconsented,
            }),
            /retention_policy_unavailable/,
          );
        }
        phase = "create Pipeline definition";
        const pipeline = (await engine.workspace({
          kind: "create_pipeline",
          workspace_id: workspace.id,
          key: randomUUID(),
          title: operation.name,
          definition,
        })) as { pipelineId: string; revision: number };
        phase = "bind owned Assets to Trail";
        const saved = (await engine.workspace({
          kind: "save_trail",
          context,
          key: randomUUID(),
          trail_id: trail.id,
          expected_revision: trail.revision,
          hypothesis: "Run the reviewed Core vector",
          pipeline: { id: pipeline.pipelineId, revision: pipeline.revision },
          bindings,
        })) as { revision: number };
        const request = {
          context,
          key: randomUUID(),
          trail_id: trail.id,
          trail_revision: saved.revision,
          pipeline_id: pipeline.pipelineId,
          pipeline_revision: pipeline.revision,
          source_asset_id: trail.source.id,
          source_revision: trail.source.revision,
          target_node_id: node,
          operation: operation.operation,
          operation_version: operation.operation_version,
          retain_output: true,
        };
        if (!worker) {
          await assert.rejects(
            engine.startRun(request),
            /native_policy_unavailable/,
          );
          return;
        }
        phase = "admit and execute Run";
        let run = (await engine.startRun(request)) as RunDetail;
        const deadline = Date.now() + 5000;
        while (run.status === "queued" || run.status === "running") {
          assert.ok(
            Date.now() < deadline,
            `worker did not finish: ${operation.operation}`,
          );
          await new Promise((resolve) => setTimeout(resolve, 20));
          run = (await engine.workspace({
            kind: "run_detail",
            context,
            run_id: run.id,
          })) as RunDetail;
        }
        assert.equal(
          run.status,
          "completed",
          `${operation.operation}: ${JSON.stringify(run.result)}`,
        );
        assert.equal(
          run.plan.contractDigest,
          fixture.contract_sha256,
          operation.operation,
        );
        assert.deepEqual(
          run.result?.outputs,
          fixture.outputs,
          operation.operation,
        );
        assert.equal(run.attempts.length, 1);
        assert.deepEqual(run.attempts[0]?.result?.outputs, fixture.outputs);
        const source = run.plan.inputs.source;
        assert.ok(source && "used" in source);
        assert.equal(source.used, assetPorts.length > 0);
        retained.push({ workspaceId: workspace.id, context, run });
        report.push({
          operation: operation.operation,
          operation_version: operation.operation_version,
          contract_sha256: fixture.contract_sha256,
          fixture_parameters: fixture.parameters,
          host_parameters: parameters,
          real_native_worker: true,
          exact_named_outputs: true,
          run_id: run.id,
        });
      }
      phase = "verify whole Run after restart";
      await verifyRestart();
      assert.equal(report.length, catalog.length);
      qualified.push(...report);
    } catch (error) {
      throw new Error(`${currentOperation}: ${phase}`, { cause: error });
    } finally {
      await engine.close();
      await rm(root, { recursive: true, force: true });
    }
  });
}
after(async () => {
  if (!worker) return;
  assert.equal(qualified.length, available.length);
  await mkdir("playwright-report", { recursive: true });
  await writeFile(
    "playwright-report/native-operation-conformance.json",
    JSON.stringify(
      {
        operations: qualified,
        unavailable: discovered
          .filter((entry) => entry.availability.state === "unavailable")
          .map((entry) => ({
            operation: entry.operation,
            availability: entry.availability,
          })),
        restart_verified: true,
      },
      null,
      2,
    ),
  );
});
