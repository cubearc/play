import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { LabCanvas } from "./LabCanvas";

let latest: { onload: (() => void) | null; onerror: (() => void) | null };
function setup(context: unknown) {
  vi.stubGlobal(
    "Image",
    class {
      onload = null;
      onerror = null;
      constructor() {
        latest = this;
      }
    },
  );
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(
    context as CanvasRenderingContext2D,
  );
}
afterEach(() => vi.unstubAllGlobals());
it("renders only a bounded crop, handles display zoom/grid and converts click positions to coordinates", () => {
  const context = {
    clearRect: vi.fn(),
    drawImage: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    strokeRect: vi.fn(),
  };
  setup(context);
  const onPick = vi.fn();
  const view = render(
    <LabCanvas
      src="/api/raster"
      width={1000}
      height={800}
      x={500}
      y={400}
      onPick={onPick}
    />,
  );
  act(() => latest.onload?.());
  expect(context.drawImage).toHaveBeenCalled();
  const canvas = screen.getByLabelText(/Pixel viewport/);
  expect(canvas).toHaveAttribute("width", "256");
  vi.spyOn(canvas, "getBoundingClientRect").mockReturnValue({
    left: 0,
    top: 0,
    width: 256,
    height: 256,
  } as DOMRect);
  fireEvent.click(canvas, { clientX: 8, clientY: 8 });
  expect(onPick).toHaveBeenCalledWith(485, 385);
  fireEvent.change(screen.getByLabelText("Pixel zoom"), {
    target: { value: "2" },
  });
  act(() => latest.onload?.());
  fireEvent.click(screen.getByLabelText(/Pixel grid/));
  act(() => latest.onload?.());
  view.rerender(
    <LabCanvas
      src="/api/raster"
      width={2}
      height={2}
      x={0}
      y={0}
      onPick={onPick}
    />,
  );
  act(() => latest.onload?.());
  onPick.mockClear();
  fireEvent.click(canvas, { clientX: 255, clientY: 255 });
  expect(onPick).not.toHaveBeenCalled();
  view.unmount();
  expect(latest.onload).toBeNull();
});
it("keeps exact controls usable after browser decoding or canvas failures", () => {
  setup(null);
  const view = render(
    <LabCanvas
      src="/api/raster"
      width={2}
      height={2}
      x={0}
      y={0}
      onPick={() => {}}
    />,
  );
  fireEvent.change(screen.getByLabelText("Pixel zoom"), {
    target: { value: "16" },
  });
  act(() => latest.onerror?.());
  expect(screen.getByRole("alert")).toHaveTextContent("unavailable");
  view.rerender(
    <LabCanvas
      src="/api/other"
      width={2}
      height={2}
      x={0}
      y={0}
      onPick={() => {}}
    />,
  );
  act(() => latest.onload?.());
  expect(screen.getByRole("alert")).toHaveTextContent("Core coordinates");
});
