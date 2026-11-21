import { expect, test } from "vitest";
import type { Operation } from "../desktop/contract";
import { initialInputs } from "./GraphInputs";

test("segmented cipher inputs start as an empty array and keep matching named slots", () => {
  const operation: Operation = {
    operation: "test.segmented",
    operation_version: 1,
    name: "Segmented",
    description: "",
    input_type: "cipher.plain_units",
    parameters: {},
    availability: { state: "available" },
    execution: { effects: "pure", inputs: { units: "json" }, outputs: {} },
    contract: {
      outputs: [],
      parameters: {},
      inputs: [
        {
          name: "units",
          semantic_type: "cipher.plain_units",
          schema: { type: "array", items: { type: "string" } },
        },
      ],
    },
  };
  expect(
    initialInputs(operation, [{ name: "source", semanticType: "text" }]),
  ).toEqual({ units: { kind: "literal", value: [] } });
  expect(
    initialInputs(operation, [
      { name: "units", semanticType: "cipher.plain_units" },
    ]),
  ).toEqual({ units: { kind: "input", name: "units" } });
});

test("optional alternatives are omitted until connected", () => {
  const operation: Operation = {
    operation: "image.qr.encode",
    operation_version: 1,
    name: "Generate a QR code",
    description: "",
    input_type: "bytes",
    parameters: { ecc: "L" },
    availability: { state: "available" },
    execution: {
      effects: "pure",
      inputs: { data: "bytes", segments: "json" },
      outputs: { modules: "bytes" },
    },
    contract: {
      outputs: [],
      parameters: {},
      inputs: [
        { name: "data", semantic_type: "bytes", schema: {}, required: false },
        {
          name: "segments",
          semantic_type: "json",
          schema: { type: "array" },
          required: false,
        },
      ],
    },
  };
  expect(
    initialInputs(operation, [{ name: "source", semanticType: "bytes" }]),
  ).toEqual({ data: { kind: "input", name: "source" } });
  expect(
    initialInputs(operation, [{ name: "source", semanticType: "text" }]),
  ).toEqual({});
});
