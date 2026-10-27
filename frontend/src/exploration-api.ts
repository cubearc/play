import { object } from "./lab-api";
import type { Store } from "./workspace-api";

export type Json =
  | null
  | boolean
  | number
  | string
  | Json[]
  | { [key: string]: Json };
export type Schema = { [key: string]: Json };
export type Port = {
  name: string;
  semantic_type: string;
  required: boolean;
  schema: Schema;
};
export type Contract = {
  schema_version: number;
  operation_version: number;
  operation: string;
  category: string;
  description: string;
  inputs: Port[];
  outputs: Port[];
  parameters: Schema;
  errors: Schema;
  policy: {
    private_input: boolean;
    private_output: boolean;
    sensitive_parameters: string[];
  };
};
export type CatalogEntry = {
  contract: Contract;
  execution: {
    available: boolean;
    reason: string | null;
    adapter_version: number;
  };
};
export type Source =
  | { type: "literal"; value: Json }
  | {
      type: "file";
      store: Store;
      path: string;
      representation: "bytes" | "text" | "image";
    }
  | { type: "artifact"; id: string; port: string }
  | { type: "step"; step: string; port: string };
export type Step = {
  id: string;
  operation: string;
  operation_version: number;
  inputs: Record<string, Source>;
  parameters: Record<string, Json>;
  private_parameters: string[];
  position: [number, number];
};
export type Exploration = {
  schema_version: 1;
  id: string;
  revision: number;
  title: string;
  hypothesis: string;
  scope: { challenge: string; stage: string } | null;
  steps: Step[];
};
export type Execution = {
  id: string;
  configuration_revision: number;
  step: string;
  status: "running" | "succeeded" | "failed" | "cancelled" | "interrupted";
  stale: boolean;
  evidence: Record<string, Json>;
  started_ms: number;
  ended_ms: number | null;
};
const invalid = () =>
  new Error("The exploration API returned an incompatible typed contract.");
function text(value: unknown): string {
  if (typeof value !== "string" || value.length > 8192) throw invalid();
  return value;
}
function integer(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
    throw invalid();
  return value;
}
function flag(value: unknown): boolean {
  if (typeof value !== "boolean") throw invalid();
  return value;
}
function array(value: unknown, limit = 2048): unknown[] {
  if (!Array.isArray(value) || value.length > limit) throw invalid();
  return value;
}
export function json(
  value: unknown,
  depth = 0,
  schemaAnnotation = false,
): Json {
  if (depth > 64) throw invalid();
  if (value === null || typeof value === "boolean" || typeof value === "string")
    return value;
  if (typeof value === "number") {
    if (
      !Number.isFinite(value) ||
      (!schemaAnnotation &&
        Number.isInteger(value) &&
        !Number.isSafeInteger(value))
    )
      throw invalid();
    return value;
  }
  if (Array.isArray(value))
    return value.map((child) => json(child, depth + 1, schemaAnnotation));
  const result: Record<string, Json> = Object.create(null);
  for (const [key, child] of Object.entries(object(value)))
    result[key] = json(child, depth + 1, schemaAnnotation);
  return result;
}
function schema(value: unknown): Schema {
  const result = object(json(value, 0, true));
  function refs(value: unknown): void {
    if (Array.isArray(value)) {
      for (const child of value) refs(child);
      return;
    }
    if (value && typeof value === "object")
      for (const [name, child] of Object.entries(value)) {
        if (
          name === "$ref" &&
          (typeof child !== "string" || !child.startsWith("#"))
        )
          throw invalid();
        refs(child);
      }
  }
  refs(result);
  return result as Schema;
}
function port(value: unknown): Port {
  const p = object(value);
  return {
    name: text(p.name),
    semantic_type: text(p.semantic_type),
    required: flag(p.required),
    schema: schema(p.schema),
  };
}
export function parseCatalog(value: unknown): CatalogEntry[] {
  const root = object(value);
  if (root.schema_version !== 2 || root.contract_schema_version !== 1)
    throw invalid();
  const ids = new Set<string>();
  return array(root.operations).map((value) => {
    const entry = object(value),
      c = object(entry.contract),
      e = object(entry.execution),
      policy = object(c.policy);
    if (
      c.schema_version !== 1 ||
      c.operation_version !== 1 ||
      e.adapter_version !== 2
    )
      throw invalid();
    const operation = text(c.operation);
    if (ids.has(operation)) throw invalid();
    ids.add(operation);
    return {
      contract: {
        schema_version: 1,
        operation_version: 1,
        operation,
        category: text(c.category),
        description: text(c.description),
        inputs: array(c.inputs, 32).map(port),
        outputs: array(c.outputs, 32).map(port),
        parameters: schema(c.parameters),
        errors: schema(c.errors),
        policy: {
          private_input: flag(policy.private_input),
          private_output: flag(policy.private_output),
          sensitive_parameters: array(policy.sensitive_parameters, 32).map(
            text,
          ),
        },
      },
      execution: {
        available: flag(e.available),
        adapter_version: 2,
        reason: e.reason === null ? null : text(e.reason),
      },
    };
  });
}
export function parseExploration(value: unknown): Exploration {
  const e = object(value);
  if (e.schema_version !== 1) throw invalid();
  const scope = e.scope === null ? null : object(e.scope);
  return {
    schema_version: 1,
    id: text(e.id),
    revision: integer(e.revision),
    title: text(e.title),
    hypothesis: text(e.hypothesis),
    scope: scope
      ? { challenge: text(scope.challenge), stage: text(scope.stage) }
      : null,
    steps: array(e.steps, 64).map((value) => {
      const s = object(value),
        inputs: Record<string, Source> = {};
      for (const [name, value] of Object.entries(object(s.inputs))) {
        const b = object(value);
        switch (b.type) {
          case "literal":
            inputs[name] = { type: "literal", value: json(b.value) };
            break;
          case "step":
            inputs[name] = {
              type: "step",
              step: text(b.step),
              port: text(b.port),
            };
            break;
          case "artifact":
            inputs[name] = {
              type: "artifact",
              id: text(b.id),
              port: text(b.port),
            };
            break;
          case "file":
            if (
              (b.store !== "source" && b.store !== "imports") ||
              !["bytes", "text", "image"].includes(String(b.representation))
            )
              throw invalid();
            inputs[name] = {
              type: "file",
              store: b.store,
              path: text(b.path),
              representation: b.representation as "bytes" | "text" | "image",
            };
            break;
          default:
            throw invalid();
        }
      }
      const position = array(s.position, 2);
      if (
        position.length !== 2 ||
        position.some((p) => typeof p !== "number" || !Number.isSafeInteger(p))
      )
        throw invalid();
      return {
        id: text(s.id),
        operation: text(s.operation),
        operation_version: integer(s.operation_version),
        inputs,
        parameters: object(json(s.parameters)) as Record<string, Json>,
        private_parameters: array(s.private_parameters ?? [], 32).map(text),
        position: position as [number, number],
      };
    }),
  };
}
async function request(
  path: string,
  method = "GET",
  body?: unknown,
  signal?: AbortSignal,
): Promise<unknown> {
  const response = await fetch(`/api${path}`, {
    method,
    credentials: "same-origin",
    ...(signal ? { signal } : {}),
    ...(body === undefined
      ? {}
      : {
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
        }),
  });
  const value: unknown = await response.json();
  if (!response.ok) {
    const error = object(object(value).error);
    throw new Error(text(error.message));
  }
  return value;
}
export async function catalog(signal: AbortSignal): Promise<CatalogEntry[]> {
  return parseCatalog(
    await request("/lab/v2/operations", "GET", undefined, signal),
  );
}
export async function explorations(
  signal: AbortSignal,
): Promise<Exploration[]> {
  return array(
    object(await request("/explorations", "GET", undefined, signal))
      .explorations,
    32,
  ).map(parseExploration);
}
export async function saveExploration(
  value: Exploration,
): Promise<Exploration> {
  return parseExploration(await request("/explorations", "PUT", value));
}
export async function executions(
  id: string,
  signal?: AbortSignal,
): Promise<Execution[]> {
  return array(
    await request(
      `/explorations/${encodeURIComponent(id)}/executions`,
      "GET",
      undefined,
      signal,
    ),
    512,
  ).map((value) => {
    const e = object(value);
    if (
      !["running", "succeeded", "failed", "cancelled", "interrupted"].includes(
        String(e.status),
      )
    )
      throw invalid();
    return {
      id: text(e.id),
      configuration_revision: integer(e.configuration_revision),
      step: text(e.step),
      status: e.status as Execution["status"],
      stale: flag(e.stale),
      evidence: object(json(e.evidence)) as Record<string, Json>,
      started_ms: integer(e.started_ms),
      ended_ms: e.ended_ms === null ? null : integer(e.ended_ms),
    };
  });
}
export async function executeStep(
  exploration: Exploration,
  step: Step,
  runs: Execution[],
  privateValues: Record<string, Json>,
  signal: AbortSignal,
): Promise<{ id: string; output: Json }> {
  const inputs: Record<string, Source> = {};
  for (const [name, source] of Object.entries(step.inputs)) {
    if (source.type !== "step") {
      inputs[name] = source;
      continue;
    }
    const run = runs
      .filter(
        (r) =>
          r.configuration_revision === exploration.revision &&
          r.step === source.step &&
          r.status === "succeeded",
      )
      .at(-1);
    if (!run || typeof run.evidence.result_id !== "string")
      throw new Error(
        "Run the connected upstream step for this configuration first.",
      );
    inputs[name] = {
      type: "artifact",
      id: run.evidence.result_id,
      port: source.port,
    };
  }
  const id = crypto.randomUUID();
  const value = object(
    await request(
      "/lab/v2/execute",
      "POST",
      {
        schema_version: 2,
        id,
        operation: step.operation,
        operation_version: step.operation_version,
        inputs,
        parameters: { ...step.parameters, ...privateValues },
        exploration: {
          id: exploration.id,
          revision: exploration.revision,
          step: step.id,
        },
      },
      signal,
    ),
  );
  if (value.schema_version !== 1) throw invalid();
  const inline = value.inline === null ? null : object(value.inline);
  return {
    id: text(object(value.result).id),
    output: inline ? json(inline.data) : null,
  };
}
export async function cancelExecution(id: string): Promise<void> {
  await fetch(`/api/lab/executions/${encodeURIComponent(id)}`, {
    method: "DELETE",
    credentials: "same-origin",
  }).then((response) => {
    if (!response.ok)
      throw new Error(
        "The execution could not be cancelled; refresh its status.",
      );
  });
}
