import type { Panel } from "./investigation-api";

type IconName = Panel | "cube" | "plus" | "arrow" | "refresh";
const paths: Record<IconName, string> = {
  cube: "m12 2 9 5v10l-9 5-9-5V7l9-5Zm-9 5 9 5 9-5M12 12v10",
  files: "M3 7V5a1 1 0 0 1 1-1h6l2 3h8a1 1 0 0 1 1 1v11H3V7Z",
  explorations: "M5 4h5v5H5zM14 15h5v5h-5zM7 9v8h7M10 6h7v9",
  notes: "M5 3h14v18H5zM8 8h8M8 12h8M8 16h5",
  artifacts: "M3 4h18v5H3zM5 9v11h14V9M9 13h6",
  solutions: "m5 12 4 4 10-10M20 12v8H4V4h9",
  challenges: "M5 21V3m0 1h14l-3 4 3 4H5",
  terminal: "m5 7 5 5-5 5m8 0h6",
  plus: "M12 5v14M5 12h14",
  arrow: "M5 12h14m-6-6 6 6-6 6",
  refresh:
    "M20 7v5h-5M4 17v-5h5M6 7a7 7 0 0 1 12-2l2 2M4 17l2 2a7 7 0 0 0 12-2",
};

export function InterfaceIcon({ name }: { name: IconName }) {
  return (
    <svg
      className="interface-icon"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d={paths[name]} />
    </svg>
  );
}

export function Brand() {
  return (
    <>
      <span className="brand-mark">
        <InterfaceIcon name="cube" />
      </span>
      <span>
        Cube<span className="brand-light">Play</span>
      </span>
    </>
  );
}
