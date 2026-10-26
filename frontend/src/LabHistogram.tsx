import { BarChart } from "echarts/charts";
import {
  AriaComponent,
  GridComponent,
  TooltipComponent,
} from "echarts/components";
import { init, use } from "echarts/core";
import { CanvasRenderer } from "echarts/renderers";
import { useEffect, useRef } from "react";
import type { Histogram } from "./lab-api";

use([BarChart, GridComponent, TooltipComponent, AriaComponent, CanvasRenderer]);
export default function LabHistogram({ data }: { data: Histogram }) {
  const target = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!target.current) return;
    const chart = init(target.current, undefined, { renderer: "canvas" });
    chart.setOption({
      animation: false,
      aria: { enabled: true },
      grid: { left: 55, right: 15, top: 15, bottom: 55 },
      tooltip: { trigger: "axis", renderMode: "richText" },
      xAxis: {
        type: "category",
        name: "Sample",
        data: data.colors.map((color) => color.values.join(",")),
      },
      yAxis: { type: "value", minInterval: 1, name: "Pixels" },
      series: [
        {
          type: "bar",
          color: "#216853",
          data: data.colors.map((color) => color.count),
        },
      ],
    });
    const observer = new ResizeObserver(() => chart.resize());
    observer.observe(target.current);
    return () => {
      observer.disconnect();
      chart.dispose();
    };
  }, [data]);
  return (
    <section aria-label="Exact histogram">
      <h4>Histogram · {data.channels.join(", ")}</h4>
      <p>
        {data.pixels.toLocaleString()} pixels. Exact Core counts; absent bins
        are omitted.
      </p>
      <div className="lab-chart" ref={target} aria-hidden="true" />
      <details>
        <summary>Histogram values</summary>
        <div className="lab-table">
          <table>
            <caption>Core sample counts</caption>
            <thead>
              <tr>
                <th scope="col">Sample</th>
                <th scope="col">Pixels</th>
              </tr>
            </thead>
            <tbody>
              {data.colors.map((color) => (
                <tr key={color.values.join(",")}>
                  <th scope="row">{color.values.join(", ")}</th>
                  <td>{color.count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
    </section>
  );
}
