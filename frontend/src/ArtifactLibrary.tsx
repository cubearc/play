import { lazy, Suspense, useEffect, useState } from "react";
import {
  execute,
  type LabResult,
  type Operation,
  object,
  operations,
  resultUrl,
  structured,
} from "./lab-api";
import { library, reopen, request, type Saved } from "./notebook-api";
import { SavedResultActions } from "./SavedResultActions";

const VisualLab = lazy(() => import("./VisualLab"));
export function ArtifactLibrary() {
  const [generation, setGeneration] = useState(0);
  return (
    <LoadedLibrary
      key={generation}
      refresh={() => setGeneration((value) => value + 1)}
    />
  );
}
function LoadedLibrary({ refresh }: { refresh: () => void }) {
  const [loading, setLoading] = useState(true);
  const [entries, setEntries] = useState<Saved[]>([]),
    [history, setHistory] = useState<Record<string, unknown>[]>([]),
    [selected, setSelected] = useState<LabResult | null>(null),
    [error, setError] = useState<string | null>(null),
    [busy, setBusy] = useState(false),
    [title, setTitle] = useState(""),
    [other, setOther] = useState(""),
    [comparison, setComparison] = useState<unknown>(null);
  useEffect(() => {
    const controller = new AbortController();
    library(controller.signal)
      .then(setEntries, (error) => {
        if (!controller.signal.aborted)
          setError(
            error instanceof Error ? error.message : "Artifacts unavailable.",
          );
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, []);
  async function open(id: string) {
    setBusy(true);
    setError(null);
    try {
      const result = await reopen(id, new AbortController().signal);
      setSelected(result);
      setTitle(entries.find((item) => item.entry.id === id)?.entry.title ?? "");
      setComparison(null);
    } catch (error) {
      setError(error instanceof Error ? error.message : "Result unavailable.");
    } finally {
      setBusy(false);
    }
  }
  const saved = entries.find((item) => item.entry.id === selected?.id);
  return (
    <section className="notebook-panel" aria-label="Artifact library">
      <h2>Artifacts and history</h2>
      <p>
        Named results and operation history remain private in this workspace.
      </p>
      <div className="lab-controls">
        <button
          type="button"
          className="button secondary"
          onClick={() => {
            setError(null);
            refresh();
          }}
        >
          Refresh artifact library
        </button>
        <button
          type="button"
          className="button secondary"
          disabled={busy}
          onClick={() => {
            setBusy(true);
            void request("/lab/history")
              .then(
                (value) => {
                  const records = object(value).records;
                  if (!Array.isArray(records) || records.length > 64)
                    throw new Error("Invalid history.");
                  setHistory(records.map(object));
                },
                (error) =>
                  setError(
                    error instanceof Error
                      ? error.message
                      : "History unavailable.",
                  ),
              )
              .finally(() => setBusy(false));
          }}
        >
          Load operation history
        </button>
      </div>
      {loading && <p role="status">Restoring artifact library…</p>}
      {!loading && !entries.length && (
        <p>
          No named artifacts yet. Select a Lab result and save it with a name.
        </p>
      )}
      <ul className="lab-results">
        {entries.map((item) => (
          <li key={item.entry.id}>
            <button
              type="button"
              className="button secondary"
              disabled={busy}
              onClick={() => void open(item.entry.id)}
            >
              {item.entry.title}
            </button>
            {item.error && (
              <p role="status">{item.error} Reference retained for recovery.</p>
            )}
          </li>
        ))}
      </ul>
      {history.length > 0 && (
        <details open>
          <summary>Retained transformation history</summary>
          <ul className="lab-results">
            {history.map((item) => (
              <li key={String(item.id)}>
                <button
                  className="button secondary"
                  type="button"
                  disabled={busy}
                  onClick={() => void open(String(item.id))}
                >
                  {String(object(item.provenance).operation)} ·{" "}
                  {String(item.representation)} · {String(item.id).slice(0, 8)}
                </button>
              </li>
            ))}
          </ul>
        </details>
      )}
      {error && (
        <p role="alert">{error} Existing records and files were preserved.</p>
      )}
      {busy && <p role="status">Opening local artifacts…</p>}
      {selected && (
        <>
          <ResultReader key={selected.id} result={selected} />
          {saved ? (
            <div className="lab-controls">
              <label>
                Saved artifact name
                <input
                  value={title}
                  maxLength={512}
                  onChange={(event) => setTitle(event.target.value)}
                />
              </label>
              <button
                type="button"
                className="button secondary"
                disabled={busy || !title.trim()}
                onClick={() => {
                  setBusy(true);
                  void request("/lab/library", undefined, {
                    id: selected.id,
                    title,
                    revision: saved.entry.revision,
                  })
                    .then(
                      () => refresh(),
                      (error) =>
                        setError(
                          error instanceof Error
                            ? error.message
                            : "Rename failed.",
                        ),
                    )
                    .finally(() => setBusy(false));
                }}
              >
                Rename saved artifact
              </button>
            </div>
          ) : null}
          <SavedResultActions key={selected.id} result={selected} />
          {selected.representation === "image" && (
            <>
              <label>
                Compare saved image
                <select
                  value={other}
                  onChange={(event) => setOther(event.target.value)}
                >
                  <option value="">Choose another saved image</option>
                  {entries
                    .filter(
                      (item) =>
                        item.entry.id !== selected.id &&
                        item.record?.representation === "image",
                    )
                    .map((item) => (
                      <option value={item.entry.id} key={item.entry.id}>
                        {item.entry.title}
                      </option>
                    ))}
                </select>
              </label>
              <button
                type="button"
                className="button secondary"
                disabled={busy || !other}
                onClick={() => {
                  setBusy(true);
                  setError(null);
                  const controller = new AbortController();
                  void (async () => {
                    const operation = (
                      await operations(controller.signal)
                    ).find((item) => item.id === "image.compare");
                    if (!operation)
                      throw new Error("Core image comparison is unavailable.");
                    const result = await execute(
                      operation,
                      { type: "artifact", id: selected.id },
                      [],
                      controller.signal,
                      false,
                      { type: "artifact", id: other },
                    );
                    setComparison(await structured(result, controller.signal));
                  })()
                    .catch((error) =>
                      setError(
                        error instanceof Error
                          ? error.message
                          : "Comparison unavailable.",
                      ),
                    )
                    .finally(() => setBusy(false));
                }}
              >
                Compare saved artifacts
              </button>
              <Suspense
                fallback={<p role="status">Loading visual controls…</p>}
              >
                <VisualLab
                  key={selected.id}
                  store="source"
                  path=""
                  initialResult={selected}
                />
              </Suspense>
            </>
          )}
          {comparison !== null && (
            <textarea
              className="text-preview"
              readOnly
              rows={8}
              aria-label="Saved artifact comparison"
              value={JSON.stringify(comparison, null, 2)}
            />
          )}
        </>
      )}
    </section>
  );
}
export function ResultReader({ result }: { result: LabResult }) {
  const [content, setContent] = useState(""),
    [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    if (result.representation === "structured")
      void structured(result, controller.signal).then(
        (value) => setContent(JSON.stringify(value, null, 2)),
        (error) => {
          if (!controller.signal.aborted)
            setError(
              error instanceof Error ? error.message : "Content unavailable.",
            );
        },
      );
    else if (result.representation === "text")
      void fetch(resultUrl(result.id), {
        credentials: "same-origin",
        signal: controller.signal,
      })
        .then(async (response) => {
          if (!response.ok) throw new Error("Text result unavailable.");
          setContent((await response.text()).slice(0, 65536));
        })
        .catch((error) => {
          if (!controller.signal.aborted)
            setError(
              error instanceof Error ? error.message : "Content unavailable.",
            );
        });
    else
      setContent(
        result.inline?.base64 ??
          "Binary content retained exactly; use explicit export to read all bytes.",
      );
    return () => controller.abort();
  }, [result]);
  return (
    <section aria-label="Reopened result">
      <h3>
        {result.semantic_type} · {result.byte_size.toLocaleString()} bytes
      </h3>
      <p>
        Artifact ID: <code>{result.id}</code>
      </p>
      <details>
        <summary>Stored provenance</summary>
        <textarea
          className="text-preview"
          readOnly
          rows={8}
          aria-label="Stored result provenance"
          value={JSON.stringify(result.provenance, null, 2)}
        />
      </details>
      {error ? (
        <p role="alert">{error}</p>
      ) : (
        <textarea
          className="text-preview"
          readOnly
          rows={8}
          aria-label="Reopened result content"
          value={content}
        />
      )}
      <p>
        Text preview is bounded to 64 KiB. Byte previews use explicit Base64
        transport; no UTF-8 conversion is inferred.
      </p>
    </section>
  );
}
export function TextLab({
  file,
}: {
  file: { store: "source" | "imports"; path: string } | null;
}) {
  const [catalog, setCatalog] = useState<Operation[]>([]),
    [operation, setOperation] = useState(""),
    [text, setText] = useState(""),
    [result, setResult] = useState<LabResult | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<string | null>(null),
    [opened, setOpened] = useState(false);
  const active = useState(() => new AbortController())[0];
  useEffect(() => () => active.abort(), [active]);
  const available = catalog.filter(
    (item) =>
      ["text", ...(file ? ["bytes"] : [])].includes(item.inputType ?? "") &&
      item.parameters.every((value) => !object(value).required),
  );
  return (
    <section className="notebook-panel" aria-label="Text and byte Lab">
      <h2>Text and byte Lab</h2>
      <p>
        Run a Core operation with its default parameters. Select a file for
        exact byte input, or type private text. Full command options remain
        available in the terminal.
      </p>
      {!opened ? (
        <button
          type="button"
          className="button secondary"
          onClick={() => {
            setOpened(true);
            void operations(active.signal).then(setCatalog, (error) =>
              setError(
                error instanceof Error
                  ? error.message
                  : "Core catalog unavailable.",
              ),
            );
          }}
        >
          Discover text and byte operations
        </button>
      ) : (
        <>
          <label>
            Core operation
            <select
              value={operation}
              onChange={(event) => setOperation(event.target.value)}
            >
              <option value="">Choose an operation</option>
              {available.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.id}
                </option>
              ))}
            </select>
          </label>
          {file ? (
            <p>
              Input file: {file.store}/{file.path} · interpreted as{" "}
              {catalog.find((item) => item.id === operation)?.inputType ??
                "the selected Core input type"}
            </p>
          ) : (
            <label>
              Private Lab text
              <textarea
                value={text}
                maxLength={65536}
                rows={5}
                onChange={(event) => setText(event.target.value)}
              />
            </label>
          )}
          <button
            type="button"
            className="button"
            disabled={busy || !operation}
            onClick={() => {
              const selected = catalog.find((item) => item.id === operation);
              if (!selected) return;
              setBusy(true);
              setError(null);
              void execute(
                selected,
                file
                  ? {
                      type: "file",
                      ...file,
                      representation:
                        selected.inputType === "text" ? "text" : "bytes",
                    }
                  : { type: "text", data: text },
                [],
                active.signal,
              )
                .then(setResult, (error) =>
                  setError(
                    error instanceof Error
                      ? error.message
                      : "Core operation failed.",
                  ),
                )
                .finally(() => setBusy(false));
            }}
          >
            Run Core operation
          </button>
        </>
      )}
      {busy && <p role="status">Running bounded Core operation…</p>}
      {error && <p role="alert">{error}</p>}
      {result && (
        <>
          <ResultReader result={result} />
          <SavedResultActions key={result.id} result={result} />
        </>
      )}
    </section>
  );
}
