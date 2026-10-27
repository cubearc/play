import type { Store } from "./workspace-api";

export type Binding = {
  name: string;
  value: number | boolean | string | { text_list: string[] };
};
export type Operation = {
  id: string;
  description: string;
  version: number;
  parameters: unknown[];
  inputType?: string;
};
export type ImageInfo = {
  width: number;
  height: number;
  color_model: string;
  channels: number;
  bit_depth: number;
  origin: string;
  format: string;
};
export type LabResult = {
  id: string;
  representation: string;
  semantic_type: string;
  byte_size: number;
  sha256: string;
  metadata: ImageInfo | null;
  provenance: Record<string, unknown>;
  retained: boolean;
  inline: { type: string; data?: unknown; base64?: string } | null;
};
export type Input =
  | {
      type: "file";
      store: Store;
      path: string;
      representation: "bytes" | "text";
    }
  | { type: "artifact"; id: string }
  | { type: "text"; data: string };
export type Histogram = {
  channels: string[];
  pixels: number;
  colors: { values: number[]; count: number }[];
};
const invalid = () =>
  new Error("The local Lab API returned an incompatible response.");
export function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw invalid();
  return value as Record<string, unknown>;
}
function text(value: unknown, maximum = 4096): string {
  if (typeof value !== "string" || value.length > maximum) throw invalid();
  return value;
}
function integer(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
    throw invalid();
  return value;
}
export function resultUrl(id: string): string {
  if (
    !/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/.test(
      id,
    )
  )
    throw invalid();
  return `/api/lab/results/${id}/content`;
}
async function response(result: Response): Promise<Record<string, unknown>> {
  const body: unknown = await result.json();
  const value = object(body);
  if (!result.ok) {
    const error = object(value.error);
    throw new Error(
      `${text(error.message, 512)}${typeof error.parameter === "string" ? ` Parameter: ${text(error.parameter, 128)}.` : ""}`,
    );
  }
  if (value.schema_version !== 1) throw invalid();
  return value;
}
export async function operations(signal: AbortSignal): Promise<Operation[]> {
  const value = await response(
    await fetch("/api/lab/operations", { signal, credentials: "same-origin" }),
  );
  if (!Array.isArray(value.operations) || value.operations.length > 2048)
    throw invalid();
  return value.operations.flatMap((item: unknown) => {
    const entry = object(item);
    if (entry.lab === null) return [];
    const core = object(entry.core),
      lab = object(entry.lab);
    if (!Array.isArray(core.parameters) || core.parameters.length > 32)
      throw invalid();
    return [
      {
        id: text(core.id, 128),
        description: text(core.description),
        version: integer(lab.version),
        parameters: core.parameters,
        ...(typeof lab.input_type === "string"
          ? { inputType: lab.input_type }
          : {}),
      },
    ];
  });
}
export function parseResult(value: Record<string, unknown>): LabResult {
  const record = object(value.result);
  const id = text(record.id, 36);
  resultUrl(id);
  if (
    typeof value.retained !== "boolean" ||
    (value.retained && value.content_url !== resultUrl(id)) ||
    (!value.retained && value.content_url !== null)
  )
    throw invalid();
  const meta =
    record.representation !== "image" || record.metadata === null
      ? null
      : object(record.metadata);
  const metadata = meta
    ? {
        width: integer(meta.width),
        height: integer(meta.height),
        color_model: text(meta.color_model, 32),
        channels: integer(meta.channels),
        bit_depth: integer(meta.bit_depth),
        origin: text(meta.origin, 128),
        format: text(meta.format, 128),
      }
    : null;
  if (
    metadata &&
    (!metadata.width ||
      !metadata.height ||
      metadata.width > 16384 ||
      metadata.height > 16384 ||
      metadata.channels > 4)
  )
    throw invalid();
  const inline = value.inline === null ? null : object(value.inline);
  return {
    id,
    representation: text(record.representation, 32),
    semantic_type: text(record.semantic_type, 128),
    byte_size: integer(record.byte_size),
    sha256: text(record.sha256, 64),
    metadata,
    provenance: object(record.provenance),
    retained: value.retained,
    inline: inline
      ? {
          type: text(inline.type, 32),
          ...(inline.data === undefined ? {} : { data: inline.data }),
          ...(inline.base64 === undefined
            ? {}
            : { base64: text(inline.base64, 6 * 1024 * 1024) }),
        }
      : null,
  };
}
export async function execute(
  operation: Operation,
  input: Input,
  parameters: Binding[],
  signal: AbortSignal,
  retain = true,
  other?: Input,
): Promise<LabResult> {
  const id = crypto.randomUUID();
  const cancel = () => {
    void fetch(`/api/lab/executions/${id}`, {
      method: "DELETE",
      credentials: "same-origin",
    }).catch(() => {});
  };
  signal.addEventListener("abort", cancel, { once: true });
  try {
    return parseResult(
      await response(
        await fetch("/api/lab/execute", {
          method: "POST",
          credentials: "same-origin",
          signal,
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            id,
            operation: operation.id,
            version: operation.version,
            input,
            parameters,
            retain_result: retain,
            ...(other ? { other } : {}),
          }),
        }),
      ),
    );
  } finally {
    signal.removeEventListener("abort", cancel);
  }
}
export async function structured(
  result: LabResult,
  signal: AbortSignal,
): Promise<unknown> {
  if (result.representation !== "structured") throw invalid();
  if (result.inline?.type === "structured") return result.inline.data;
  const response = await fetch(resultUrl(result.id), {
    signal,
    credentials: "same-origin",
  });
  if (!response.ok)
    throw new Error(
      "The structured result is unavailable. Retry the operation.",
    );
  return response.json();
}
export function histogram(value: unknown): Histogram {
  const data = object(value);
  if (
    !Array.isArray(data.channels) ||
    data.channels.length > 4 ||
    !Array.isArray(data.colors) ||
    data.colors.length > 10000
  )
    throw invalid();
  return {
    pixels: integer(data.pixels),
    channels: data.channels.map((v: unknown) => text(v, 32)),
    colors: data.colors.map((v: unknown) => {
      const color = object(v);
      if (!Array.isArray(color.values) || color.values.length > 4)
        throw invalid();
      return {
        values: color.values.map((v: unknown) => {
          const n = integer(v);
          if (n > 255) throw invalid();
          return n;
        }),
        count: integer(color.count),
      };
    }),
  };
}
export function pixelBytes(result: LabResult): number[] {
  if (
    result.inline?.type !== "bytes" ||
    !result.inline.base64 ||
    result.inline.base64.length > 8
  )
    throw invalid();
  return Array.from(atob(result.inline.base64), (value) => value.charCodeAt(0));
}

export function channelUrl(data: unknown): string {
  const value = object(data).preview_png_base64;
  if (
    typeof value !== "string" ||
    value.length > 6 * 1024 * 1024 ||
    !/^[A-Za-z0-9+/]*={0,2}$/.test(value)
  )
    throw new Error("The channel preview is incompatible.");
  return `data:image/png;base64,${value}`;
}
