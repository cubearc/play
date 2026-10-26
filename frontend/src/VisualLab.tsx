import { lazy, Suspense, useEffect, useRef, useState } from "react";
import { LabCanvas } from "./LabCanvas";

const LabHistogram = lazy(() => import("./LabHistogram"));

import {
  type Binding,
  channelUrl,
  execute,
  type Histogram,
  histogram,
  type LabResult,
  type Operation,
  operations,
  pixelBytes,
  resultUrl,
  structured,
} from "./lab-api";
import { SavedResultActions } from "./SavedResultActions";
import type { Store } from "./workspace-api";

function channels(model: string): string[] {
  switch (model) {
    case "gray":
      return ["gray"];
    case "gray_alpha":
      return ["gray", "alpha"];
    case "rgb":
      return ["red", "green", "blue"];
    case "rgba":
      return ["red", "green", "blue", "alpha"];
    case "indexed":
      return ["index"];
    default:
      return [];
  }
}
export default function VisualLab({
  initialResult,
  store,
  path,
}: {
  store: Store;
  path: string;
  initialResult?: LabResult;
}) {
  const [catalog, setCatalog] = useState<Operation[]>([]);
  const [original, setOriginal] = useState<LabResult | null>(
    initialResult ?? null,
  );
  const [image, setImage] = useState<LabResult | null>(initialResult ?? null);
  const [results, setResults] = useState<LabResult[]>(
    initialResult ? [initialResult] : [],
  );
  const [selected, setSelected] = useState<LabResult | null>(
    initialResult ?? null,
  );
  const [content, setContent] = useState<unknown>(null);
  const [counts, setCounts] = useState<Histogram | null>(null);
  const [pixel, setPixel] = useState<number[] | null>(null);
  const [channelPreview, setChannelPreview] = useState<string | null>(null);
  const [channel, setChannel] = useState(
    initialResult?.metadata
      ? (channels(initialResult.metadata.color_model)[0] ?? "")
      : "",
  );
  const [x, setX] = useState(0),
    [y, setY] = useState(0);
  const [busy, setBusy] = useState(false),
    [error, setError] = useState<string | null>(null);
  const active = useRef<AbortController | null>(null);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      active.current?.abort();
    };
  }, []);
  async function job(task: (signal: AbortSignal) => Promise<void>) {
    if (active.current) return;
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setError(null);
    try {
      await task(controller.signal);
    } catch (error: unknown) {
      if (mounted.current)
        setError(
          controller.signal.aborted
            ? "Operation cancelled. You can retry when ready."
            : error instanceof Error
              ? error.message
              : "The Lab operation could not finish.",
        );
    } finally {
      active.current = null;
      if (mounted.current) setBusy(false);
    }
  }
  function find(id: string, entries = catalog) {
    const operation = entries.find((entry) => entry.id === id);
    if (!operation) throw new Error(`This Core version does not expose ${id}.`);
    return operation;
  }
  function choose(result: LabResult, data: unknown = null) {
    setSelected(result);
    setContent(data);
    if (result.metadata && result.representation === "image") {
      setImage(result);
      setChannel(channels(result.metadata.color_model)[0] ?? "");
      setCounts(null);
      setPixel(null);
      setChannelPreview(null);
      setX(0);
      setY(0);
    } else {
      setChannelPreview(data === null ? null : channelUrl(data));
    }
  }
  function retain(result: LabResult, data: unknown = null) {
    if (!mounted.current) return;
    setResults((previous) => [...previous, result]);
    choose(result, data);
  }
  async function open() {
    await job(async (signal) => {
      const entries = await operations(signal);
      const decoded = await execute(
        find("image.decode", entries),
        { type: "file", store, path, representation: "bytes" },
        [],
        signal,
      );
      if (!mounted.current) return;
      setCatalog(entries);
      setOriginal(decoded);
      retain(decoded);
    });
  }
  useEffect(() => {
    if (!initialResult) return;
    const controller = new AbortController();
    operations(controller.signal).then(setCatalog, (error) => {
      if (!controller.signal.aborted)
        setError(
          error instanceof Error ? error.message : "Core catalog unavailable.",
        );
    });
    return () => controller.abort();
  }, [initialResult]);
  const info = image?.metadata;
  const availableChannels = info ? channels(info.color_model) : [];
  const input = image ? { type: "artifact" as const, id: image.id } : null;
  const ready = !busy && Boolean(input) && catalog.length > 0;
  const validPixel = Boolean(
    info &&
      Number.isInteger(x) &&
      Number.isInteger(y) &&
      x >= 0 &&
      y >= 0 &&
      x < info.width &&
      y < info.height,
  );
  async function probe(kind: "pixels" | "histogram" | "channel") {
    if (!input) return;
    await job(async (signal) => {
      if (kind === "pixels") {
        const result = await execute(
          find("image.pixels.read"),
          input,
          [
            { name: "x", value: x },
            { name: "y", value: y },
          ],
          signal,
          false,
        );
        if (mounted.current) {
          setPixel(pixelBytes(result));
          setSelected(result);
        }
      } else if (kind === "histogram") {
        const result = await execute(
          find("image.histogram"),
          input,
          [{ name: "channels", value: { text_list: [channel] } }],
          signal,
          false,
        );
        const data = await structured(result, signal);
        if (mounted.current) {
          setCounts(histogram(data));
          setContent(data);
          setSelected(result);
        }
      } else {
        const result = await execute(
          find("image.channel.extract"),
          input,
          [{ name: "channel", value: channel }],
          signal,
        );
        const data = await structured(result, signal);
        if (mounted.current) retain(result, data);
      }
    });
  }
  async function transform(id: string, parameters: Binding[] = []) {
    if (!input) return;
    await job(async (signal) => {
      const result = await execute(find(id), input, parameters, signal);
      retain(result);
    });
  }
  return (
    <section className="visual-lab" aria-label="Visual Lab">
      <h3>Visual Lab</h3>
      <p>
        Core-backed analysis. Files and generated results stay in this
        workspace.
      </p>
      {!original && (
        <button
          className="button"
          type="button"
          disabled={busy}
          onClick={() => void open()}
        >
          Decode image in Core
        </button>
      )}
      {busy && (
        <div role="status">
          Running a bounded Core operation…{" "}
          <button
            type="button"
            className="button secondary"
            onClick={() => active.current?.abort()}
          >
            Cancel operation
          </button>
        </div>
      )}
      {error && (
        <p role="alert">
          {error} Sources remain unchanged. Retry the requested action.
        </p>
      )}
      {original && image && info && (
        <>
          <dl className="file-facts">
            <div>
              <dt>Decoded raster</dt>
              <dd>
                {info.width} × {info.height} · {info.color_model} ·{" "}
                {info.bit_depth} bits
              </dd>
            </div>
            <div>
              <dt>Representation</dt>
              <dd>
                {info.format} / {info.origin}; displayed as Core PNG
              </dd>
            </div>
          </dl>
          <p className="file-notice">
            Coordinates are zero-based from the top left of the Core raster.
            Browser color management and zoom are presentation only; values
            below come from Core, not canvas samples. Indexed pixels remain
            palette indices until explicitly expanded.
          </p>
          <div className="lab-comparison">
            <figure>
              <img
                className="image-preview"
                src={resultUrl(original.id)}
                alt="Original Core-decoded raster"
              />
              <figcaption>Original decoded raster</figcaption>
            </figure>
            <figure>
              <img
                className="image-preview"
                src={resultUrl(image.id)}
                alt="Active Lab raster"
              />
              <figcaption>
                Active image · {String(image.provenance.operation)}
              </figcaption>
            </figure>
          </div>
          <fieldset disabled={busy}>
            <legend>Exact pixel inspection</legend>
            <LabCanvas
              src={resultUrl(image.id)}
              width={info.width}
              height={info.height}
              x={validPixel ? x : 0}
              y={validPixel ? y : 0}
              onPick={(px, py) => {
                setX(px);
                setY(py);
                setPixel(null);
              }}
            />
            <div className="lab-controls">
              <label>
                Pixel X
                <input
                  type="number"
                  min={0}
                  max={info.width - 1}
                  value={Number.isNaN(x) ? "" : x}
                  onChange={(event) => {
                    setX(event.target.valueAsNumber);
                    setPixel(null);
                  }}
                />
              </label>
              <label>
                Pixel Y
                <input
                  type="number"
                  min={0}
                  max={info.height - 1}
                  value={Number.isNaN(y) ? "" : y}
                  onChange={(event) => {
                    setY(event.target.valueAsNumber);
                    setPixel(null);
                  }}
                />
              </label>
              <button
                type="button"
                className="button secondary"
                disabled={!ready || !validPixel}
                onClick={() => void probe("pixels")}
              >
                Read pixel
              </button>
            </div>
          </fieldset>
          {pixel && (
            <p role="status">
              Core pixel ({x}, {y}) · {availableChannels.join(", ")}:{" "}
              {pixel.join(", ")}
            </p>
          )}
          <fieldset disabled={busy}>
            <legend>Channels and counts</legend>
            <div className="lab-controls">
              <label>
                Analysis channel
                <select
                  value={channel}
                  onChange={(event) => {
                    setChannel(event.target.value);
                    setCounts(null);
                    setChannelPreview(null);
                  }}
                >
                  {availableChannels.map((value) => (
                    <option key={value}>{value}</option>
                  ))}
                </select>
              </label>
              <button
                type="button"
                className="button secondary"
                disabled={!ready || !channel}
                onClick={() => void probe("histogram")}
              >
                Compute histogram
              </button>
              <button
                type="button"
                className="button secondary"
                disabled={!ready || !channel}
                onClick={() => void probe("channel")}
              >
                Extract channel
              </button>
            </div>
          </fieldset>
          {counts && (
            <Suspense fallback={<p role="status">Loading histogram…</p>}>
              <LabHistogram data={counts} />
            </Suspense>
          )}
          {channelPreview && (
            <figure>
              <img
                className="image-preview"
                src={channelPreview}
                alt="Core-generated grayscale channel preview"
              />
              <figcaption>
                Extracted sample intensity; Core grayscale PNG
              </figcaption>
            </figure>
          )}
          <fieldset disabled={busy}>
            <legend>Transform active image</legend>
            <div className="lab-controls">
              {[
                "image.invert",
                "image.grayscale",
                "image.rotate",
                "image.mirror",
                ...(info.color_model === "indexed"
                  ? ["image.palette.expand"]
                  : []),
              ].map((id) => {
                const operation = catalog.find((entry) => entry.id === id);
                return (
                  operation && (
                    <button
                      type="button"
                      className="button secondary"
                      title={operation.description}
                      key={id}
                      disabled={!ready}
                      onClick={() =>
                        void transform(
                          id,
                          id === "image.rotate"
                            ? [{ name: "degrees", value: 90 }]
                            : id === "image.mirror"
                              ? [{ name: "axis", value: "horizontal" }]
                              : [],
                        )
                      }
                    >
                      {id === "image.rotate"
                        ? "Rotate 90°"
                        : id === "image.mirror"
                          ? "Mirror horizontally"
                          : id === "image.palette.expand"
                            ? "Expand palette to RGBA"
                            : id === "image.invert"
                              ? "Invert"
                              : "Grayscale"}
                    </button>
                  )
                );
              })}
            </div>
            <p>
              Transforms create separate results. Rotation: 90 degrees; mirror:
              horizontal; other effective parameters appear below.
            </p>
          </fieldset>
          <button
            type="button"
            className="button secondary"
            disabled={!ready || image.id === original.id}
            onClick={() => {
              void job(async (signal) => {
                const result = await execute(
                  find("image.compare"),
                  { type: "artifact", id: original.id },
                  [],
                  signal,
                  false,
                  input ?? undefined,
                );
                const data = await structured(result, signal);
                if (mounted.current) {
                  setContent(data);
                  setSelected(result);
                }
              });
            }}
          >
            Compare with original
          </button>
          <p>
            Exact comparison requires compatible dimensions and sample models.
            Core rejects incompatible pairs; results are never silently
            resampled.
          </p>
          <section aria-label="Generated Lab results">
            <h4>Generated results</h4>
            <p>
              Select a result to inspect it or continue transforming an image.
              Results are retained locally; named artifact history is a later
              capability.
            </p>
            <ul className="lab-results">
              {results.map((result, index) => (
                <li key={result.id}>
                  <button
                    type="button"
                    className="button secondary"
                    disabled={busy}
                    aria-pressed={selected?.id === result.id}
                    onClick={() => {
                      void job(async (signal) => {
                        const data =
                          result.representation === "structured"
                            ? await structured(result, signal)
                            : null;
                        if (mounted.current) choose(result, data);
                      });
                    }}
                  >
                    {index + 1}. {String(result.provenance.operation)} ·{" "}
                    {result.representation}
                  </button>
                </li>
              ))}
            </ul>
          </section>
          {selected && (
            <details open>
              <summary>Selected result and provenance</summary>
              <p>
                {selected.retained ? "Selected artifact" : "Transient probe"}:{" "}
                <code>{selected.id}</code> ·{" "}
                {selected.byte_size.toLocaleString()} bytes ·{" "}
                {selected.semantic_type}
              </p>
              <textarea
                className="text-preview"
                aria-label="Result provenance"
                readOnly
                rows={10}
                value={JSON.stringify(
                  {
                    id: selected.id,
                    sha256: selected.sha256,
                    metadata: selected.metadata,
                    provenance: selected.provenance,
                  },
                  null,
                  2,
                )}
              />
            </details>
          )}
          {selected?.retained && (
            <SavedResultActions key={selected.id} result={selected} />
          )}
          {content !== null && (
            <section aria-label="Structured Core result">
              <h4>Structured Core result</h4>
              <textarea
                className="text-preview"
                aria-label="Structured result content"
                readOnly
                rows={10}
                value={JSON.stringify(content, null, 2)}
              />
            </section>
          )}
        </>
      )}
    </section>
  );
}
