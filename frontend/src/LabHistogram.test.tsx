import { render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import LabHistogram from "./LabHistogram";
import { counts } from "./test/lab-fixture";

const chart = vi.hoisted(() => ({
  setOption: vi.fn(),
  resize: vi.fn(),
  dispose: vi.fn(),
}));
vi.mock("echarts/core", () => ({ use: vi.fn(), init: vi.fn(() => chart) }));
afterEach(() => vi.unstubAllGlobals());
it("passes exact tuples to ECharts, exposes semantic counts, resizes and disposes", () => {
  let resize = () => {};
  const disconnect = vi.fn();
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(callback: () => void) {
        resize = callback;
      }
      observe() {}
      disconnect = disconnect;
    },
  );
  const view = render(<LabHistogram data={counts} />);
  expect(screen.getByRole("table", { hidden: true })).toHaveTextContent(
    "Core sample counts",
  );
  expect(
    screen.getByRole("rowheader", { name: "20", hidden: true }),
  ).toBeInTheDocument();
  expect(chart.setOption.mock.calls[0]?.[0].series[0].data).toEqual([
    1, 1, 1, 1,
  ]);
  resize();
  expect(chart.resize).toHaveBeenCalled();
  view.unmount();
  expect(disconnect).toHaveBeenCalled();
  expect(chart.dispose).toHaveBeenCalled();
});
