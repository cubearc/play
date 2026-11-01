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
