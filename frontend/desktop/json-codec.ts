/** Preserve native JSON integers across JavaScript, private IPC and inert viewers. */
import type { JsonValue } from "./workspace-contract.ts";

declare global {
  interface JSON {
    rawJSON(value: string): unknown;
  }
}
const minimum = -(1n << 63n);
const maximum = (1n << 64n) - 1n;
function checked(value: bigint): bigint {
  if (value < minimum || value > maximum)
    throw new Error("integer_out_of_range");
  return value;
}
function raw(value: string): unknown {
  if (typeof JSON.rawJSON !== "function")
    throw new Error("lossless_json_unavailable");
  return JSON.rawJSON(value);
}
export function parseJson(document: string): JsonValue {
  return JSON.parse(
    document,
    (_key: string, value: unknown, context?: { source?: string }) => {
      if (typeof value !== "number") return value;
      if (!Number.isFinite(value)) throw new Error("invalid_json_value");
      if (value === 0 && context?.source === "-0") return 0;
      if (Number.isInteger(value) && !Number.isSafeInteger(value)) {
        if (!context?.source) throw new Error("lossless_json_unavailable");
        if (/^-?(0|[1-9][0-9]*)$/.test(context.source))
          return checked(BigInt(context.source));
      }
      return value;
    },
  );
}
export function stringifyJson(
  value: unknown,
  _replacer: null = null,
  space?: string | number,
): string {
  const serialized = JSON.stringify(
    value,
    (_key, item: unknown) => {
      if (typeof item === "bigint") return raw(checked(item).toString());
      if (typeof item === "number") {
        if (!Number.isFinite(item)) throw new Error("invalid_json_value");
        if (Object.is(item, -0)) return raw("-0.0");
        // Preserve the floating representation instead of relabelling it as an integer.
        if (Number.isInteger(item) && !Number.isSafeInteger(item))
          return raw(item.toExponential());
      }
      return item;
    },
    space,
  );
  if (serialized === undefined) throw new Error("invalid_json_value");
  return serialized;
}

export function isExactInteger(value: unknown): value is number | bigint {
  return typeof value === "bigint"
    ? value >= minimum && value <= maximum
    : typeof value === "number" && Number.isSafeInteger(value);
}
export function parseInteger(raw: string): number | bigint {
  if (!/^-?(0|[1-9][0-9]*)$/.test(raw))
    throw new Error("Enter an exact decimal integer.");
  const value = parseJson(raw);
  if (!isExactInteger(value)) throw new Error("integer_out_of_range");
  return value;
}
