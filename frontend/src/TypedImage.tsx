import { useEffect, useMemo, useRef, useState } from "react";
import type { Json } from "./exploration-api";
import { object } from "./lab-api";

type Raster = {
  width: number;
  height: number;
  channels: string[];
  samples: Uint8Array;
  rgba: Uint8ClampedArray;
};
const invalid = () => new Error("Unsupported exact 8-bit raster shape.");
function bytes(value: unknown): Uint8Array {
  const text = object(value).base64;
  if (typeof text !== "string" || text.length > 24 * 1024 * 1024)
    throw invalid();
  return Uint8Array.from(atob(text), (c) => c.charCodeAt(0));
}
export function raster(value: unknown): Raster {
  const v = object(value);
  if (v.bit_depth !== undefined && v.bit_depth !== 8) throw invalid();
  const width = v.width,
    height = v.height;
  if (
    typeof width !== "number" ||
    typeof height !== "number" ||
    !Number.isSafeInteger(width) ||
    !Number.isSafeInteger(height) ||
    width < 1 ||
    height < 1 ||
    width > 16384 ||
    height > 16384 ||
    width * height > 4194304
  )
    throw invalid();
  const models: Record<string, string[]> = {
    gray: ["gray"],
    gray_alpha: ["gray", "alpha"],
    rgb: ["red", "green", "blue"],
    rgba: ["red", "green", "blue", "alpha"],
    indexed: ["index"],
  };
  const model = String(v.color_model);
  const channels = Object.hasOwn(models, model) ? models[model] : undefined;
  if (!channels) throw invalid();
  const samples = bytes(v.samples),
    transparency = bytes(v.transparency);
  const palette = v.palette;
  if (
    samples.length !== width * height * channels.length ||
    !Array.isArray(palette) ||
    palette.length > 256 ||
    palette.some(
      (entry) =>
        !Array.isArray(entry) ||
        entry.length !== 3 ||
        entry.some(
          (v) =>
            typeof v !== "number" || !Number.isInteger(v) || v < 0 || v > 255,
        ),
    )
  )
    throw invalid();
  if (
    model === "indexed" &&
    (!palette.length || transparency.length > palette.length)
  )
    throw invalid();
  if (
    model !== "indexed" &&
    transparency.length > 0 &&
    (transparency.length !== (model === "gray" ? 2 : model === "rgb" ? 6 : 0) ||
      transparency.some((v, i) => i % 2 === 0 && v !== 0))
  )
    throw invalid();
  const rgba = new Uint8ClampedArray(width * height * 4);
  for (let pixel = 0; pixel < width * height; pixel++) {
    const offset = pixel * channels.length,
      r = samples[offset] ?? 0;
    let color: number[];
    if (model === "indexed") {
      if (r >= palette.length) throw invalid();
      color = [...palette[r], transparency[r] ?? 255];
    } else if (model === "gray" || model === "gray_alpha") {
      color = [
        r,
        r,
        r,
        model === "gray_alpha"
          ? (samples[offset + 1] ?? 255)
          : transparency.length && transparency[1] === r
            ? 0
            : 255,
      ];
    } else {
      const g = samples[offset + 1] ?? 0,
        b = samples[offset + 2] ?? 0;
      color = [
        r,
        g,
        b,
        model === "rgba"
          ? (samples[offset + 3] ?? 255)
          : transparency.length &&
              transparency[1] === r &&
              transparency[3] === g &&
              transparency[5] === b
            ? 0
            : 255,
      ];
    }
    rgba.set(color, pixel * 4);
  }
  return { width, height, channels, samples, rgba };
}
export function TypedImage({ value }: { value: Json }) {
  const image = useMemo(() => {
    try {
      return raster(value);
    } catch {
      return null;
    }
  }, [value]);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [error, setError] = useState(false),
    [zoom, setZoom] = useState(1);
  const [x, setX] = useState(0),
    [y, setY] = useState(0);
  useEffect(() => {
    if (!image) return;
    const context = canvas.current?.getContext("2d");
    if (!context) {
      setError(true);
      return;
    }
    try {
      const frame = context.createImageData(image.width, image.height);
      frame.data.set(image.rgba);
      context.imageSmoothingEnabled = false;
      context.putImageData(frame, 0, 0);
      setError(false);
    } catch {
      setError(true);
    }
  }, [image]);
  if (!image)
    return (
      <p>
        This raster has unsupported metadata or sample shape. Inspect the
        bounded output data instead.
      </p>
    );
  const pixel = image.samples.slice(
    (y * image.width + x) * image.channels.length,
    (y * image.width + x + 1) * image.channels.length,
  );
  return (
    <section aria-label="Exact raster viewer">
      <p>
        {image.width} × {image.height} · 8-bit native{" "}
        {image.channels.join(", ")} samples.
      </p>
      <p>
        Canvas is a display preview. The table reads original Core samples
        before browser color management; display colors imply no forensic
        precision.
      </p>
      <label>
        Pixel zoom
        <input
          type="range"
          min="1"
          max="16"
          value={zoom}
          onChange={(event) => setZoom(Number(event.target.value))}
        />
      </label>
      {error && (
        <p role="status">
          Canvas preview unavailable. Exact sample inspection remains available
          below.
        </p>
      )}
      <div className="lab-image-scroll">
        <canvas
          ref={canvas}
          width={image.width}
          height={image.height}
          role="img"
          aria-label="Core sample display preview"
          style={{
            width: image.width * zoom,
            height: image.height * zoom,
            imageRendering: "pixelated",
          }}
        />
      </div>
      <label>
        Pixel x
        <input
          type="number"
          min="0"
          max={image.width - 1}
          value={x}
          onChange={(event) => {
            const value = Number(event.target.value);
            if (Number.isInteger(value) && value >= 0 && value < image.width)
              setX(value);
          }}
        />
      </label>
      <label>
        Pixel y
        <input
          type="number"
          min="0"
          max={image.height - 1}
          value={y}
          onChange={(event) => {
            const value = Number(event.target.value);
            if (Number.isInteger(value) && value >= 0 && value < image.height)
              setY(value);
          }}
        />
      </label>
      <table>
        <caption>
          Exact native samples at ({x}, {y})
        </caption>
        <thead>
          <tr>
            {image.channels.map((c) => (
              <th key={c} scope="col">
                {c}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          <tr>
            {Array.from(pixel, (v, i) => (
              <td key={image.channels[i]}>{v}</td>
            ))}
          </tr>
        </tbody>
      </table>
    </section>
  );
}
