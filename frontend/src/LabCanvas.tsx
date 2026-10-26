import { useEffect, useRef, useState } from "react";

// Only presentation: authoritative samples are requested separately from Cube Core.
export function LabCanvas({
  src,
  width,
  height,
  x,
  y,
  onPick,
}: {
  src: string;
  width: number;
  height: number;
  x: number;
  y: number;
  onPick: (x: number, y: number) => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [zoom, setZoom] = useState(8);
  const [grid, setGrid] = useState(true);
  const [failed, setFailed] = useState(false);
  const size = 256;
  const columns = Math.min(width, Math.ceil(size / zoom)),
    rows = Math.min(height, Math.ceil(size / zoom));
  const left = Math.max(
    0,
    Math.min(width - columns, x - Math.floor(columns / 2)),
  );
  const top = Math.max(0, Math.min(height - rows, y - Math.floor(rows / 2)));
  useEffect(() => {
    let active = true;
    const image = new Image();
    setFailed(false);
    image.onload = () => {
      if (!active) return;
      const context = canvas.current?.getContext("2d");
      if (!context) {
        setFailed(true);
        return;
      }
      context.clearRect(0, 0, size, size);
      context.imageSmoothingEnabled = false;
      context.drawImage(
        image,
        left,
        top,
        columns,
        rows,
        0,
        0,
        columns * zoom,
        rows * zoom,
      );
      if (grid && zoom >= 8) {
        context.strokeStyle = "#65716b";
        context.lineWidth = 0.5;
        context.beginPath();
        for (let column = 0; column <= columns; column++) {
          context.moveTo(column * zoom, 0);
          context.lineTo(column * zoom, rows * zoom);
        }
        for (let row = 0; row <= rows; row++) {
          context.moveTo(0, row * zoom);
          context.lineTo(columns * zoom, row * zoom);
        }
        context.stroke();
      }
      context.strokeStyle = "#d35400";
      context.lineWidth = 2;
      context.strokeRect(
        (x - left) * zoom + 1,
        (y - top) * zoom + 1,
        zoom - 2,
        zoom - 2,
      );
    };
    image.onerror = () => {
      if (active) setFailed(true);
    };
    image.src = src;
    return () => {
      active = false;
      image.onload = null;
      image.onerror = null;
    };
  }, [src, zoom, grid, left, top, columns, rows, x, y]);
  return (
    <div className="lab-pixels">
      <label>
        Pixel zoom
        <select
          value={zoom}
          onChange={(event) => setZoom(Number(event.target.value))}
        >
          {[2, 8, 16, 32].map((value) => (
            <option key={value} value={value}>
              {value}×
            </option>
          ))}
        </select>
      </label>
      <label>
        <input
          type="checkbox"
          checked={grid}
          onChange={(event) => setGrid(event.target.checked)}
        />{" "}
        Pixel grid (8× or higher)
      </label>
      <p>
        Viewport origin ({left}, {top}). Use coordinates below to inspect any
        pixel.
      </p>
      <canvas
        ref={canvas}
        width={size}
        height={size}
        aria-label="Pixel viewport; use coordinate controls for keyboard inspection"
        onClick={(event) => {
          const rect = event.currentTarget.getBoundingClientRect();
          const px =
              left +
              Math.floor(
                ((event.clientX - rect.left) * size) / rect.width / zoom,
              ),
            py =
              top +
              Math.floor(
                ((event.clientY - rect.top) * size) / rect.height / zoom,
              );
          if (px < width && py < height) onPick(px, py);
        }}
      />
      {failed && (
        <p role="alert">
          Pixel rendering is unavailable. Core coordinates and values remain
          available.
        </p>
      )}
    </div>
  );
}
