import { type FormEvent, useEffect, useRef, useState } from "react";
import { FileInspector } from "./FileInspector";
import {
  importFile,
  type Listing,
  listFiles,
  type Store,
} from "./workspace-api";

const IMPORT_LIMIT = 32 * 1024 * 1024;
type Selected = { store: Store; path: string };

export function FileWorkspace({
  initialSelection,
}: {
  initialSelection: { name: string; intent: "file" | "challenge" } | null;
}) {
  const [query, setQuery] = useState<{
    store: Store;
    path: string;
    offset: number;
  }>({ store: "source", path: "", offset: 0 });
  const { store, path, offset } = query;
  const [listing, setListing] = useState<Listing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Selected | null>(() =>
    initialSelection ? { store: "source", path: initialSelection.name } : null,
  );
  const [file, setFile] = useState<File | null>(null);
  const [name, setName] = useState("");
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const upload = useRef<AbortController | null>(null);
  const input = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    setListing(null);
    setError(null);
    listFiles(query.store, query.path, query.offset, controller.signal).then(
      (value) => {
        if (controller.signal.aborted) return;
        setListing(value);
        const supported = value.entries.filter(
          (entry) =>
            entry.accessible &&
            entry.kind === "file" &&
            entry.category !== "unknown",
        );
        if (
          query.offset === 0 &&
          value.next_offset === null &&
          !value.truncated &&
          supported.length === 1
        ) {
          const candidate = supported[0]?.path;
          if (candidate)
            setSelected(
              (previous) => previous ?? { store: query.store, path: candidate },
            );
        }
      },
      (error: unknown) => {
        if (!controller.signal.aborted)
          setError(
            error instanceof Error
              ? error.message
              : "The directory is unavailable.",
          );
      },
    );
    return () => controller.abort();
  }, [query]);
  useEffect(() => () => upload.current?.abort(), []);

  function navigate(nextStore: Store, nextPath: string) {
    setQuery({ store: nextStore, path: nextPath, offset: 0 });
    setSelected(null);
  }
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!file || importing) return;
    const controller = new AbortController();
    upload.current = controller;
    setImporting(true);
    setImportError(null);
    try {
      const saved = await importFile(file, name, controller.signal);
      if (controller.signal.aborted) return;
      setQuery({ store: saved.store, path: "", offset: 0 });
      setSelected(saved);

      setFile(null);
      if (input.current) input.current.value = "";
    } catch (error) {
      if (!controller.signal.aborted)
        setImportError(
          error instanceof Error
            ? error.message
            : "The import could not finish.",
        );
    } finally {
      if (upload.current === controller) {
        upload.current = null;
        setImporting(false);
      }
    }
  }

  return (
    <section className="file-workspace" aria-label="Workspace files">
      <div className="files-toolbar">
        <div className="source-controls">
          <button
            type="button"
            className="button secondary"
            aria-pressed={store === "source"}
            onClick={() => navigate("source", "")}
          >
            Source files
          </button>
          <button
            type="button"
            className="button secondary"
            aria-pressed={store === "imports"}
            disabled={listing !== null && !listing.imports_available}
            onClick={() => navigate("imports", "")}
          >
            Imported files
          </button>
        </div>
        <button
          type="button"
          className="button secondary"
          onClick={() => setQuery((value) => ({ ...value }))}
        >
          Refresh files
        </button>
      </div>
      <div className="file-layout">
        <aside className="file-explorer" aria-label="File explorer">
          <nav className="file-breadcrumbs" aria-label="Directory">
            <button type="button" onClick={() => navigate(store, "")}>
              {store === "source" ? "Source" : "Imports"}
            </button>
            {path
              .split("/")
              .filter(Boolean)
              .map((component, index, components) => (
                <span key={components.slice(0, index + 1).join("/")}>
                  <span aria-hidden="true"> / </span>
                  <button
                    type="button"
                    onClick={() =>
                      navigate(store, components.slice(0, index + 1).join("/"))
                    }
                  >
                    {component}
                  </button>
                </span>
              ))}
          </nav>
          {path && (
            <button
              type="button"
              className="directory-up"
              aria-label="Up one directory"
              onClick={() =>
                navigate(store, path.split("/").slice(0, -1).join("/"))
              }
            >
              ↑ Up one directory
            </button>
          )}
          {error ? (
            <p className="file-warning" role="alert">
              {error}
            </p>
          ) : !listing ? (
            <p role="status">Loading files…</p>
          ) : (
            <>
              {listing.entries.length === 0 ? (
                <div className="files-empty">
                  <h3>
                    {store === "source"
                      ? "This workspace is empty"
                      : "No imported files yet"}
                  </h3>
                  <p>
                    Import a file below to begin. Source files are never
                    changed.
                  </p>
                </div>
              ) : (
                <ul className="file-list">
                  {listing.entries.map((entry) => (
                    <li key={entry.name}>
                      <button
                        type="button"
                        className={
                          selected?.store === store &&
                          selected.path === entry.path
                            ? "file-item selected"
                            : "file-item"
                        }
                        disabled={!entry.accessible}
                        aria-label={`${entry.kind === "directory" ? "Open directory" : "Inspect"} ${entry.name}`}
                        aria-pressed={
                          entry.kind === "file"
                            ? selected?.store === store &&
                              selected.path === entry.path
                            : undefined
                        }
                        onClick={() => {
                          if (!entry.path) return;
                          if (entry.kind === "directory")
                            navigate(store, entry.path);
                          else setSelected({ store, path: entry.path });
                        }}
                      >
                        <span className="file-symbol" aria-hidden="true">
                          {entry.kind === "directory" ? "▰" : "▤"}
                        </span>
                        <span className="file-item-copy">
                          <span className="file-name">{entry.name}</span>
                          <span className="file-item-detail">
                            {entry.accessible
                              ? entry.kind === "directory"
                                ? "Directory"
                                : `${entry.category} · ${entry.size?.toLocaleString()} bytes`
                              : `${entry.kind} · not opened`}
                          </span>
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
              {listing.truncated && (
                <p className="file-warning" role="status">
                  Discovery stopped at {listing.entry_limit.toLocaleString()}{" "}
                  entries. Open a narrower directory to inspect the rest; no
                  recursive scan was performed.
                </p>
              )}
              {(offset > 0 || listing.next_offset !== null) && (
                <div className="page-controls">
                  <button
                    type="button"
                    className="button secondary"
                    disabled={offset === 0}
                    onClick={() =>
                      setQuery((value) => ({
                        ...value,
                        offset: Math.max(0, offset - listing.page_size),
                      }))
                    }
                  >
                    Previous files
                  </button>
                  <button
                    type="button"
                    className="button secondary"
                    disabled={listing.next_offset === null}
                    onClick={() => {
                      if (listing.next_offset !== null)
                        setQuery((value) => ({
                          ...value,
                          offset: listing.next_offset ?? value.offset,
                        }));
                    }}
                  >
                    Next files
                  </button>
                </div>
              )}
            </>
          )}
        </aside>
        {selected ? (
          <FileInspector
            key={`${selected.store}:${selected.path}`}
            store={selected.store}
            path={selected.path}
          />
        ) : (
          <div className="inspector-empty">
            <p className="eyebrow">FOLLOW THE EVIDENCE</p>
            <h2>Select a file to inspect</h2>
            <p>
              Preview supported files or inspect their original bytes. Filename
              categories are hints; analysis begins only when you select a file.
            </p>
          </div>
        )}
      </div>
      <section className="import-panel" aria-labelledby="import-title">
        <div>
          <h2 id="import-title">Import a local file</h2>
          <p>
            Copies only the file you choose into private workspace storage.
            Maximum 32 MiB. Nothing is uploaded to a cloud service.
          </p>
        </div>
        {listing && !listing.imports_available ? (
          <p>
            Managed storage is unavailable. Reopen this directory with cube
            play.
          </p>
        ) : (
          <>
            <label className="file-picker">
              Choose a file
              <input
                ref={input}
                type="file"
                disabled={importing}
                onChange={(event) => {
                  const chosen = event.target.files?.[0] ?? null;
                  setFile(chosen);
                  setName(chosen?.name ?? "");
                  setImportError(null);
                }}
              />
            </label>
            {file && (
              <form className="import-form" onSubmit={submit}>
                <label htmlFor="import-name">
                  Save as
                  <input
                    id="import-name"
                    required
                    maxLength={255}
                    value={name}
                    disabled={importing}
                    onChange={(event) => setName(event.target.value)}
                  />
                </label>
                <span>{file.size.toLocaleString()} bytes</span>
                {file.size > IMPORT_LIMIT && (
                  <p role="alert">This file exceeds the 32 MiB import limit.</p>
                )}
                <button
                  className="button"
                  type="submit"
                  disabled={importing || !name || file.size > IMPORT_LIMIT}
                >
                  {importing ? "Importing…" : "Import into workspace"}
                </button>
                {importing && (
                  <button
                    className="button secondary"
                    type="button"
                    onClick={() => {
                      upload.current?.abort();
                      setImportError(
                        "Import canceled. Refresh files if the server had already completed it.",
                      );
                    }}
                  >
                    Cancel import
                  </button>
                )}
              </form>
            )}
          </>
        )}
        {importError && (
          <p role="alert" className="file-warning">
            {importError}
          </p>
        )}
      </section>
      {initialSelection?.intent === "challenge" && (
        <p className="file-notice">
          Challenge definitions are inspected as data here. Stage import and
          navigation are provided by the separate challenge capability; no
          challenge content is executed.
        </p>
      )}
    </section>
  );
}
