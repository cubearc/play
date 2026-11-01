import type { JsonValue } from "../desktop/workspace-contract";

export function parseExactJson(raw: string): JsonValue {
  return JSON.parse(
    raw,
    (_key: string, value: unknown, context?: { source?: string }) => {
      if (
        typeof value === "number" &&
        Number.isInteger(value) &&
        (!Number.isSafeInteger(value) ||
          (context?.source && !/^-?(0|[1-9][0-9]*)$/.test(context.source)))
      )
        throw new Error(
          "Use exact integer values here. Use Parse JSON for decimal or exponent data.",
        );
      return value;
    },
  );
}
