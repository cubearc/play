import { describe, expect, it } from "vitest";
import type { Operation } from "../../desktop/contract";
import { parameterTypesValid } from "../ParameterFields";

const operation: Operation = {
  operation: "test.contract",
  operation_version: 1,
  name: "Contract",
  description: "",
  input_type: "text",
  parameters: { traversal: "row" },
  contract: {
    inputs: [],
    outputs: [],
    parameters: {
      type: "object",
      required: [],
      additionalProperties: false,
      properties: {
        weight: { type: "number", minimum: 0, maximum: 1 },
        traversal: { type: "string", enum: ["row", "spiral_cw"] },
        channels: {
          type: "array",
          minItems: 1,
          maxItems: 4,
          uniqueItems: true,
          items: { type: "string", enum: ["gray", "red"] },
        },
        region: {
          type: "array",
          minItems: 4,
          maxItems: 4,
          items: { type: "integer", minimum: 0, maximum: 4294967295 },
        },
      },
    },
  },
};
describe("native contract-driven parameters", () => {
  it("validates enums and omitted optional controls without an operation-specific UI", () => {
    expect(parameterTypesValid(operation, { traversal: "spiral_cw" })).toBe(
      true,
    );
    expect(parameterTypesValid(operation, { traversal: "shell" })).toBe(false);
  });
  it("requires well-typed, unique array choices and an exact region tuple", () => {
    expect(
      parameterTypesValid(operation, {
        traversal: "row",
        channels: ["gray"],
        region: [0, 0, 3, 3],
      }),
    ).toBe(true);
    for (const channels of [[], ["gray", "gray"], ["blue"], "["])
      expect(
        parameterTypesValid(operation, { traversal: "row", channels }),
      ).toBe(false);
    for (const region of [
      [0, 0],
      [-1, 0, 3, 3],
      [0, 0, 3.5, 3],
    ])
      expect(parameterTypesValid(operation, { traversal: "row", region })).toBe(
        false,
      );
  });
  it("keeps finite numeric parameters typed and bounded", () => {
    expect(
      parameterTypesValid(operation, { traversal: "row", weight: 0.5 }),
    ).toBe(true);
    for (const weight of ["0.5", NaN, Infinity, -0.5, 1.5])
      expect(parameterTypesValid(operation, { traversal: "row", weight })).toBe(
        false,
      );
  });
  it("rejects unknown options and missing required values before Run admission", () => {
    const required: Operation = {
      ...operation,
      contract: {
        ...operation.contract,
        parameters: {
          type: "object",
          required: ["count", "enabled", "options"],
          properties: {
            count: { type: "integer" },
            enabled: { type: "boolean" },
            options: { type: "object" },
          },
        },
      },
    };
    const values = { count: 3, enabled: false, options: {} };
    expect(parameterTypesValid(required, values)).toBe(true);
    for (const count of [3.5, "3", Number.MAX_SAFE_INTEGER + 1])
      expect(parameterTypesValid(required, { ...values, count })).toBe(false);
    for (const options of [null, [], "{}"])
      expect(parameterTypesValid(required, { ...values, options })).toBe(false);
    expect(parameterTypesValid(required, { ...values, enabled: "false" })).toBe(
      false,
    );
    expect(parameterTypesValid(required, { ...values, count: undefined })).toBe(
      false,
    );
    expect(parameterTypesValid(required, { ...values, extra: true })).toBe(
      false,
    );
  });
  it("permits a no-parameter contract but never an undeclared option", () => {
    const empty: Operation = {
      ...operation,
      parameters: {},
      contract: { ...operation.contract, parameters: null },
    };
    expect(parameterTypesValid(empty, {})).toBe(true);
    expect(parameterTypesValid(empty, { hidden: true })).toBe(false);
  });
});
