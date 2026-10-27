import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { CaptureImport } from "./CaptureImport";
import { importCapture } from "./exploration-api";
import { parseResult } from "./lab-api";
import { id, outputPort, typedWire } from "./test/lab-fixture";

vi.mock("./exploration-api", async (original) => ({
  ...(await original<typeof import("./exploration-api")>()),
  importCapture: vi.fn(),
}));
it("imports only the explicit selected file, reports rejection and allows a deliberate retry", async () => {
  vi.mocked(importCapture)
    .mockRejectedValueOnce(new Error("Invalid capture digest"))
    .mockResolvedValueOnce({
      id,
      record: parseResult(typedWire()),
      ports: [outputPort],
      output: { text: "A6" },
    });
  const user = userEvent.setup();
  render(<CaptureImport asset={{ store: "source", path: "capture.json" }} />);
  expect(importCapture).not.toHaveBeenCalled();
  await user.click(
    screen.getByText("Import a selected CLI capture", { exact: true }),
  );
  await user.click(
    screen.getByRole("button", {
      name: "Import selected CLI capture",
    }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Invalid capture digest",
  );
  await user.click(
    screen.getByRole("button", {
      name: "Import selected CLI capture",
    }),
  );
  expect(await screen.findByLabelText("Exact text output")).toHaveValue("A6");
  expect(importCapture).toHaveBeenLastCalledWith(
    "source",
    "capture.json",
    expect.any(AbortSignal),
  );
});
