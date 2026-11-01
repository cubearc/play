import { expect, it } from "vitest";
import { operationLabel } from "./operationLabel";

it("distinguishes both directions when Core gives a profile the same canonical name", () => {
  const encode = { name: "VIC (English 2010)", operation: "cipher.vic.encode" };
  const decode = { ...encode, operation: "cipher.vic.decode" };
  expect(operationLabel(encode, [encode, decode])).toBe(
    "Encode VIC (English 2010)",
  );
  expect(operationLabel(decode, [encode, decode])).toBe(
    "Decode VIC (English 2010)",
  );
});
it("preserves unique names and operations with no encode/decode direction", () => {
  const rot13 = { name: "ROT13", operation: "cipher.rot13" };
  const encode = { name: "Encode UTF-8", operation: "text.utf8.encode" };
  expect(operationLabel(rot13, [rot13, encode])).toBe("ROT13");
  expect(
    operationLabel(rot13, [
      rot13,
      { ...rot13, operation: "custom.profile.apply" },
    ]),
  ).toBe("ROT13");
  expect(operationLabel(encode, [rot13, encode])).toBe("Encode UTF-8");
});
