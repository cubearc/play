import { expect, test } from "vitest";
import type { Operation } from "../desktop/contract";
import { initialInputs } from "./GraphInputs";

test("segmented cipher inputs start as an empty array and keep matching named slots", () => {
  const operation = {
    contract: {
      inputs: [
        {
          name: "units",
          semantic_type: "cipher.plain_units",
          schema: { type: "array", items: { type: "string" } },
        },
      ],
    },
  } as Operation;
  expect(
    initialInputs(operation, [{ name: "source", semanticType: "text" }]),
  ).toEqual({ units: { kind: "literal", value: [] } });
  expect(
    initialInputs(operation, [
      { name: "units", semanticType: "cipher.plain_units" },
    ]),
  ).toEqual({ units: { kind: "input", name: "units" } });
});
