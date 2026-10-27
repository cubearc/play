import { apiFetch } from "./capability";
import { object } from "./lab-api";
import { request } from "./notebook-api";
import type { Store } from "./workspace-api";
export type Stage = {
  id: string;
  title: string;
  instructions: string;
  steps: { id: string; instructions: string }[];
  hints: string[];
  artifacts: string[];
  answer_type: "text" | "bytes";
};
export type ChallengeState = {
  challenges: { key: string; id: string; title: string }[];
  active: null | {
    definition: {
      key: string;
      store: Store;
      path: string;
      manifest: {
        id: string;
        title: string;
        description: string;
        stages: Stage[];
      };
    };
    stage: string;
    revision: number;
    artifacts: { id: string; store: Store; path: string }[];
    progress: {
      stage_key: string;
      revision: number;
      local_status: "unstarted" | "in_progress" | "completed";
    }[];
  };
};
function text(value: unknown, max = 8192): string {
  if (typeof value !== "string" || value.length > max)
    throw new Error("Incompatible local challenge text.");
  return value;
}
function array<T>(
  value: unknown,
  max: number,
  parse: (value: unknown) => T,
): T[] {
  if (!Array.isArray(value) || value.length > max)
    throw new Error("Incompatible challenge list.");
  return value.map(parse);
}
function number(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
    throw new Error("Incompatible challenge revision.");
  return value;
}
export function parseChallenge(value: unknown): ChallengeState {
  const state = object(value);
  if (state.schema_version !== 1)
    throw new Error("Unsupported local challenge API.");
  const choices = array(state.challenges, 16, (value) => {
    const item = object(value);
    return {
      key: text(item.key, 64),
      id: text(item.id, 64),
      title: text(item.title, 512),
    };
  });
  if (state.active === null) return { challenges: choices, active: null };
  const active = object(state.active),
    definition = object(active.definition),
    manifest = object(definition.manifest);
  const store = (value: unknown): Store => {
    if (value !== "source" && value !== "imports")
      throw new Error("Invalid challenge file store.");
    return value;
  };
  return {
    challenges: choices,
    active: {
      definition: {
        key: text(definition.key, 64),
        store: store(definition.store),
        path: text(definition.path, 4096),
        manifest: {
          id: text(manifest.id, 64),
          title: text(manifest.title, 512),
          description: text(manifest.description),
          stages: array(manifest.stages, 32, (value) => {
            const stage = object(value);
            if (stage.answer_type !== "text" && stage.answer_type !== "bytes")
              throw new Error("Unsupported stage answer type.");
            return {
              id: text(stage.id, 64),
              title: text(stage.title, 512),
              instructions: text(stage.instructions),
              steps: array(stage.steps, 16, (value) => {
                const step = object(value);
                return {
                  id: text(step.id, 64),
                  instructions: text(step.instructions),
                };
              }),
              hints: array(stage.hints, 16, (value) => text(value)),
              artifacts: array(stage.artifacts, 32, (value) => text(value, 64)),
              answer_type: stage.answer_type,
            };
          }),
        },
      },
      stage: text(active.stage, 64),
      revision: number(active.revision),
      artifacts: array(active.artifacts, 32, (value) => {
        const artifact = object(value);
        return {
          id: text(artifact.id, 64),
          store: store(artifact.store),
          path: text(artifact.path, 4096),
        };
      }),
      progress: array(active.progress, 32, (value) => {
        const progress = object(value);
        if (
          !["unstarted", "in_progress", "completed"].includes(
            String(progress.local_status),
          )
        )
          throw new Error("Invalid local progress.");
        return {
          stage_key: text(progress.stage_key, 64),
          revision: number(progress.revision),
          local_status: progress.local_status as
            | "unstarted"
            | "in_progress"
            | "completed",
        };
      }),
    },
  };
}
export async function challengeRequest(
  path = "",
  payload?: unknown,
  signal?: AbortSignal,
): Promise<ChallengeState> {
  return parseChallenge(await request(`/challenges${path}`, signal, payload));
}
export async function importChallenge(
  file: { store: Store; path: string },
  signal?: AbortSignal,
): Promise<ChallengeState> {
  const response = await apiFetch("/api/challenges/import", {
    method: "POST",
    credentials: "omit",
    ...(signal ? { signal } : {}),
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(file),
  });
  const value: unknown = await response.json();
  if (!response.ok)
    throw new Error(
      String(
        object(value).message ??
          "Challenge import failed; no definition was executed.",
      ),
    );
  return parseChallenge(value);
}
