import assert from "node:assert/strict";
import test from "node:test";
import type { Operation, Representation } from "./contract.ts";
import {
  assetAcceptsInputType,
  assetInputTypes,
} from "./workspace-contract.ts";

const capability = (
  semanticType: string,
  representation: Representation,
): Operation => ({
  operation: `synthetic.${representation}`,
  operation_version: 1,
  name: "Synthetic capability",
  description: "",
  input_type: semanticType,
  parameters: {},
  availability: { state: "available" },
  execution: {
    effects: "pure",
    inputs: { source: representation },
    outputs: {},
  },
  contract: {
    inputs: [{ name: "source", semantic_type: semanticType, schema: {} }],
    outputs: [],
    parameters: {},
  },
});
const file = {
  content: {
    kind: "file" as const,
    byteLength: 0,
    sha256: "",
    preview: { format: "hex" as const, text: "", truncated: false },
  },
};

test("new Core semantic types use declared lossless Asset views without an operation-ID branch", () => {
  const operations = [
    capability("custom.binary", "bytes"),
    capability("custom.tokens", "byte_array"),
    capability("custom.prose", "text"),
  ];
  assert.equal(assetAcceptsInputType(file, "custom.binary", operations), true);
  assert.equal(assetAcceptsInputType(file, "custom.tokens", operations), true);
  assert.equal(assetAcceptsInputType(file, "custom.prose", operations), false);
  assert.equal(
    assetAcceptsInputType(
      { content: { kind: "text", text: "" } },
      "custom.binary",
      operations,
    ),
    false,
  );
  assert.equal(
    assetAcceptsInputType(
      { content: { kind: "page", text: "" } },
      "custom.prose",
      operations,
    ),
    true,
  );
  assert.equal(
    assetAcceptsInputType({ content: { kind: "image" } }, "custom.picture", [
      capability("custom.picture", "image_samples"),
    ]),
    true,
  );
});

test("ambiguous or absent types never gain Asset authority through a label", () => {
  for (const operations of [
    [],
    [capability("custom.binary", "bytes"), capability("custom.binary", "text")],
  ]) {
    assert.equal(assetInputTypes(operations).has("custom.binary"), false);
    assert.equal(
      assetAcceptsInputType(file, "custom.binary", operations),
      false,
    );
  }
});

test("typed structured Assets bind only their exact Core semantic type", () => {
  const operations = [
    capability("custom.record", "json"),
    capability("other.record", "json"),
  ];
  const asset = {
    content: {
      kind: "structured" as const,
      semanticType: "custom.record",
      value: { score: 7 },
    },
  };
  assert.equal(assetAcceptsInputType(asset, "custom.record", operations), true);
  assert.equal(assetAcceptsInputType(asset, "other.record", operations), false);
  assert.equal(assetAcceptsInputType(file, "custom.record", operations), false);
});
