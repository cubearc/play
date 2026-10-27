export const id = "6d434f94-89f3-4fb8-a7d8-c345a0000001";
export const metadata = {
  width: 2,
  height: 2,
  color_model: "rgb",
  channels: 3,
  bit_depth: 8,
  origin: "decoded",
  format: "png",
};
export const counts = {
  channels: ["red"],
  pixels: 4,
  colors: [
    { values: [20], count: 1 },
    { values: [40], count: 1 },
    { values: [120], count: 1 },
    { values: [230], count: 1 },
  ],
};
export function wire(
  operation = "image.decode",
  overrides: Record<string, unknown> = {},
) {
  return {
    schema_version: 1,
    retained: true,
    content_url: `/api/lab/results/${id}/content`,
    result: {
      id,
      representation: "image",
      semantic_type: "image.samples",
      byte_size: 80,
      sha256: "0".repeat(64),
      metadata,
      provenance: { operation, parameters: [], core_version: "0.18.0" },
    },
    inline: null,
    ...overrides,
  };
}
export function catalog(
  ids = [
    "image.decode",
    "image.pixels.read",
    "image.histogram",
    "image.channel.extract",
    "image.invert",
    "image.rotate",
    "image.mirror",
    "image.grayscale",
    "image.compare",
    "image.palette.expand",
  ],
) {
  return {
    schema_version: 1,
    operations: ids.map((id) => ({
      core: { id, description: `Core ${id}`, parameters: [] },
      lab: { version: 1 },
    })),
  };
}

export const outputPort = {
  name: "text",
  semantic_type: "text",
  schema_version: 1,
  operation_version: 1,
  schema_sha256: "1".repeat(64),
};
export function typedWire(output: Record<string, unknown> = { text: "A6" }) {
  return wire("cipher.rot5", {
    result: {
      ...wire().result,
      representation: "structured",
      semantic_type: "operation.outputs",
      metadata: { schema_version: 2, ports: [outputPort] },
      provenance: {
        operation: "cipher.rot5",
        parameters: [],
        core_version: "0.18.0",
      },
    },
    inline: { type: "structured", data: output },
  });
}
