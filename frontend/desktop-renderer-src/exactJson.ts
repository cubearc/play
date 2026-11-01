import { parseJson } from "../desktop/json-codec";
import type { JsonValue } from "../desktop/workspace-contract";

export function parseExactJson(raw: string): JsonValue {
  return JSON.parse(
    raw,
    (_key: string, value: unknown, context?: { source?: string }) => {
      if (
        typeof value === "number" &&
        Number.isInteger(value) &&
        context?.source &&
        !/^-?(0|[1-9][0-9]*)$/.test(context.source)
      )
        throw new Error(
          "Use exact integer values here. Use Parse JSON for decimal or exponent data.",
        );
      return typeof value === "number" && context?.source
        ? parseJson(context.source)
        : value;
    },
  );
}
