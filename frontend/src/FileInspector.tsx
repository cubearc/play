import { lazy, Suspense, useEffect, useState } from "react";
import { useReadLink } from "./useReadLink";
import {
  type Inspection,
  inspectFile,
  mediaUrl,
  type Store,
} from "./workspace-api";

const VisualLab = lazy(() => import("./VisualLab"));

export function FileInspector({ store, path }: { store: Store; path: string }) {
  const [labOpen, setLabOpen] = useState(false);
  const [request, setRequest] = useState({ offset: 0 });
  const offset = request.offset;
  const [result, setResult] = useState<Inspection | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [mediaFailed, setMediaFailed] = useState(false);
  useEffect(() => {
    const controller = new AbortController();
    setResult(null);
    setError(null);
    setMediaFailed(false);
    inspectFile(store, path, request.offset, controller.signal).then(
      (value) => {
        if (!controller.signal.aborted) setResult(value);
      },
      (error: unknown) => {
        if (!controller.signal.aborted)
          setError(
            error instanceof Error
              ? error.message
              : "The selected file is unavailable.",
          );
      },
    );
    return () => controller.abort();
  }, [store, path, request]);
  const preview = useReadLink(
    result?.preview.available &&
      ["image", "audio", "video"].includes(result.preview.kind)
      ? mediaUrl(store, path)
      : null,
  );
  const name = path.split("/").at(-1) ?? path;
  return (
    <section className="file-inspector" aria-label="File inspector">
      <div className="inspector-heading">
        <div>
          <p className="eyebrow">FILE INSPECTOR</p>
          <h2>{name}</h2>
        </div>
        <span className="file-origin">
          {store === "source" ? "Read-only source" : "Private imported copy"}
        </span>
      </div>
      {error ? (
        <div className="file-message" role="alert">
          <p>{error}</p>
          <button
            className="button secondary"
            type="button"
            onClick={() => setRequest((value) => ({ ...value }))}
          >
            Retry inspection
          </button>
        </div>
      ) : !result ? (
        <p role="status">Inspecting the selected file…</p>
      ) : (
        <>
          <dl className="file-facts">
            <div>
              <dt>Size</dt>
              <dd>{result.entry.size?.toLocaleString()} bytes</dd>
            </div>
            <div>
              <dt>Filename hint</dt>
              <dd>{result.entry.category}</dd>
            </div>
            <div>
              <dt>Modified</dt>
              <dd>
                {result.entry.modified_ms === null
                  ? "Unavailable"
                  : new Date(result.entry.modified_ms).toISOString()}
              </dd>
            </div>
            {result.image && (
              <div>
                <dt>Raster</dt>
                <dd>
                  {result.image.width} × {result.image.height} pixels
                </dd>
              </div>
            )}
          </dl>
          {result.extension_mismatch && (
            <p className="file-warning" role="status">
              Header evidence differs from the filename extension. Signatures
              are candidates, not proof of validity.
            </p>
          )}
          {result.signatures.length > 0 && (
            <p className="signature-evidence">
              Core header evidence:{" "}
              {result.signatures.map((item) => item.format).join(", ")}.{" "}
              {result.signatures[0]?.rule}.
            </p>
          )}
          {result.notice && <p className="file-notice">{result.notice}</p>}
          {(mediaFailed || preview.error) && (
            <div>
              <p className="file-warning" role="status">
                The media preview failed or its private link expired. Metadata
                and byte inspection remain available below.
              </p>
              <button
                type="button"
                className="button secondary"
                onClick={() => {
                  setMediaFailed(false);
                  preview.renew();
                }}
              >
                Retry media preview
              </button>
            </div>
          )}
          {preview.error && <p role="alert">{preview.error}</p>}
          {result.preview.available &&
            !mediaFailed &&
            preview.url &&
            result.preview.kind === "image" && (
              <img
                className="image-preview"
                src={preview.url ?? undefined}
                alt={`Preview of ${name}`}
                onError={() => setMediaFailed(true)}
              />
            )}
          {result.preview.available &&
            !mediaFailed &&
            preview.url &&
            result.preview.kind === "video" && (
              <>
                <video
                  className="media-preview"
                  controls
                  muted
                  preload="metadata"
                  src={preview.url ?? undefined}
                  onError={() => setMediaFailed(true)}
                  aria-label={`Video preview of ${name}`}
                />
                <p className="file-notice">
                  Media starts muted. Use the playback controls to enable sound.
                </p>
              </>
            )}
          {result.preview.available &&
            !mediaFailed &&
            preview.url &&
            result.preview.kind === "audio" && (
              <>
                <audio
                  className="media-preview"
                  controls
                  muted
                  preload="metadata"
                  src={preview.url ?? undefined}
                  onError={() => setMediaFailed(true)}
                  aria-label={`Audio preview of ${name}`}
                />
                <p className="file-notice">
                  Media starts muted. Use the playback controls to enable sound.
                </p>
              </>
            )}
          {result.text && (
            <section aria-label="Text preview">
              <h3>UTF-8 text</h3>
              <textarea
                className="text-preview"
                aria-label="Text content"
                readOnly
                rows={12}
                value={result.text.value}
              />
              {!result.text.value && <p>(Empty text file)</p>}
              {result.text.truncated && (
                <p className="file-notice">
                  Showing {result.text.bytes.toLocaleString()} bytes within the
                  64 KiB text limit. Original bytes remain unchanged.
                </p>
              )}
            </section>
          )}
          {result.media && (
            <details>
              <summary>Cube container metadata</summary>
              <textarea
                className="text-preview"
                aria-label="Container metadata content"
                readOnly
                rows={10}
                value={JSON.stringify(result.media, null, 2)}
              />
            </details>
          )}
          {(result.preview.kind === "image" ||
            result.entry.category === "image") &&
            (labOpen ? (
              <Suspense
                fallback={<p role="status">Loading visual controls…</p>}
              >
                <VisualLab store={store} path={path} />
              </Suspense>
            ) : (
              <button
                type="button"
                className="button"
                onClick={() => setLabOpen(true)}
              >
                Open visual Lab
              </button>
            ))}
          <section aria-label="Byte inspection">
            <div className="bytes-heading">
              <h3>Original bytes</h3>
              <span className="bytes-label">
                Offset {result.offset.toLocaleString()} · hexadecimal / ASCII
              </span>
            </div>
            {result.rows.length ? (
              <textarea
                className="byte-rows"
                aria-label="Hexadecimal rows"
                readOnly
                rows={12}
                value={result.rows
                  .map((row) => `${row.offset}  ${row.hex}  ${row.ascii}`)
                  .join("\n")}
              />
            ) : (
              <p>No bytes in this file.</p>
            )}
            <div className="page-controls">
              <button
                type="button"
                className="button secondary"
                disabled={offset === 0}
                onClick={() =>
                  setRequest({ offset: Math.max(0, offset - 4096) })
                }
              >
                Previous bytes
              </button>
              <button
                type="button"
                className="button secondary"
                disabled={result.next_offset === null}
                onClick={() => {
                  if (result.next_offset !== null)
                    setRequest({ offset: result.next_offset });
                }}
              >
                Next bytes
              </button>
            </div>
          </section>
        </>
      )}
    </section>
  );
}
