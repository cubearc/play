import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { raster, TypedImage } from "./TypedImage";

afterEach(() => vi.restoreAllMocks());
function logical(model = "rgb", samples = [1, 2, 3, 10, 20, 30], width = 2) {
  return {
    width,
    height: 1,
    color_model: model,
    samples: { base64: btoa(String.fromCharCode(...samples)) },
    palette: [] as number[][],
    transparency: { base64: "" },
  };
}
it("renders original Core sample values with keyboard coordinates and bounded pixel zoom", async () => {
  const putImageData = vi.fn();
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
    createImageData: (w: number, h: number) => ({
      data: new Uint8ClampedArray(w * h * 4),
    }),
    putImageData,
  } as unknown as CanvasRenderingContext2D);
  const user = userEvent.setup();
  render(<TypedImage value={logical()} />);
  expect(putImageData).toHaveBeenCalled();
  expect(screen.getByRole("table")).toHaveTextContent("1");
  await user.clear(screen.getByLabelText("Pixel x"));
  await user.type(screen.getByLabelText("Pixel x"), "1");
  const cells = within(screen.getByRole("table")).getAllByRole("cell");
  expect(cells.map((c) => c.textContent)).toEqual(["10", "20", "30"]);
  await user.click(screen.getByLabelText("Pixel zoom"));
  expect(
    screen.getByRole("img", { name: "Core sample display preview" }),
  ).toBeVisible();
});
it("preserves native palette indices, transparency and all supported display layouts", () => {
  const indexed = {
    ...logical("indexed", [1]),
    palette: [
      [1, 2, 3],
      [4, 5, 6],
    ],
    transparency: { base64: btoa(String.fromCharCode(0, 127)) },
    width: 1,
  };
  expect(Array.from(raster(indexed).samples)).toEqual([1]);
  expect(Array.from(raster(indexed).rgba)).toEqual([4, 5, 6, 127]);
  for (const [value, rgba] of [
    [logical("gray", [7], 1), [7, 7, 7, 255]],
    [logical("gray_alpha", [7, 2], 1), [7, 7, 7, 2]],
    [logical("rgba", [1, 2, 3, 4], 1), [1, 2, 3, 4]],
    [
      {
        ...logical("gray", [7], 1),
        transparency: { base64: btoa(String.fromCharCode(0, 7)) },
      },
      [7, 7, 7, 0],
    ],
    [
      {
        ...logical("rgb", [1, 2, 3], 1),
        transparency: { base64: btoa(String.fromCharCode(0, 1, 0, 2, 0, 3)) },
      },
      [1, 2, 3, 0],
    ],
  ] as const)
    expect(Array.from(raster(value).rgba)).toEqual(rgba);
});
it("fails closed on unsupported metadata and retains sample inspection without Canvas", () => {
  for (const value of [
    { ...logical(), bit_depth: 16 },
    { ...logical(), width: 0 },
    { ...logical(), width: 16385 },
    { ...logical(), width: 1 },
    { ...logical(), color_model: "constructor" },
    { ...logical(), palette: [[1, 2, 999]] },
    logical("indexed", [0], 1),
    { ...logical(), transparency: { base64: "AQ==" } },
    { ...logical(), samples: { base64: "!!!" } },
    { ...logical("indexed", [1], 1), palette: [[1, 2, 3]] },
  ])
    expect(() => raster(value)).toThrow();
  const view = render(<TypedImage value={{ width: 0 }} />);
  expect(screen.getByText(/unsupported metadata/)).toBeVisible();
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
  view.rerender(<TypedImage value={logical()} />);
  expect(screen.getByRole("status")).toHaveTextContent(
    "Exact sample inspection remains available",
  );
  expect(screen.getByRole("table")).toBeVisible();
});
