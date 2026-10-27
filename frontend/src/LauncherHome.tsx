import { useEffect, useRef, useState } from "react";
import {
  openWorkspace,
  type RecentWorkspace,
  recentWorkspaces,
  type WorkspaceIntent,
} from "./launcher-api";

const navigateToWorkspace = (url: string) => window.location.replace(url);

export function LauncherHome({
  initialLink = null,
  initialError = null,
  navigate = navigateToWorkspace,
}: {
  initialLink?: string | null;
  initialError?: string | null;
  navigate?: (url: string) => void;
}) {
  const [loaded, setLoaded] = useState<{
    generation: number;
    items: RecentWorkspace[];
  } | null>(null);
  const [name, setName] = useState("Investigation"),
    [error, setError] = useState<string | null>(initialError),
    [status, setStatus] = useState(
      initialError
        ? "Workspace chooser unavailable."
        : "Loading local workspaces…",
    ),
    [busy, setBusy] = useState(false),
    [generation, setGeneration] = useState(0);
  const items = loaded?.generation === generation ? loaded.items : [];
  const active = useRef<AbortController | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    if (!initialError || generation > 0)
      recentWorkspaces(controller.signal)
        .then((items) => {
          if (controller.signal.aborted) return;
          setLoaded({ generation, items });
          setStatus("Choose a local investigation.");
          if (initialLink) {
            setStatus("Opening the ready workspace…");
            navigate(initialLink);
          }
        })
        .catch((error: unknown) => {
          if (!controller.signal.aborted) {
            setError(
              error instanceof Error
                ? error.message
                : "Local workspaces are unavailable.",
            );
            setStatus("Recent workspaces could not be loaded.");
          }
        });
    return () => controller.abort();
  }, [generation, initialError, initialLink, navigate]);
  useEffect(() => () => active.current?.abort(), []);
  async function open(intent: WorkspaceIntent) {
    if (busy) return;
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setError(null);
    setStatus(
      intent.intent === "recent"
        ? "Restoring the local workspace…"
        : "Choose a folder in the system dialog. The workspace will open after startup.",
    );
    try {
      const link = await openWorkspace(intent, controller.signal);
      if (!controller.signal.aborted) {
        if (link) navigate(link);
        else
          setStatus(
            "Folder selection cancelled. Existing investigations were preserved.",
          );
      }
    } catch (error) {
      if (!controller.signal.aborted) {
        setError(
          error instanceof Error
            ? error.message
            : "The workspace could not be opened.",
        );
        setStatus("Existing investigation data was preserved.");
      }
    } finally {
      if (!controller.signal.aborted) setBusy(false);
      active.current = null;
    }
  }
  return (
    <div className="app-shell">
      <header className="app-header">
        <span className="brand">
          Cube<span className="brand-light">Play</span>
        </span>
        <span className="privacy-badge">Local by design</span>
      </header>
      <main className="launcher-home">
        <p className="eyebrow">FOLLOW THE EVIDENCE</p>
        <h1>Your investigations</h1>
        <p>
          Start with an empty workspace, open a local directory or resume an
          investigation.
        </p>
        <div className="launcher-actions">
          <section>
            <h2>New workspace</h2>
            <p>
              Choose its parent folder in the system dialog. Notes and generated
              results stay separate from source files.
            </p>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void open({ intent: "new", name });
              }}
            >
              <label htmlFor="workspace-name">Workspace folder name</label>
              <input
                id="workspace-name"
                value={name}
                maxLength={128}
                required
                disabled={busy}
                onChange={(e) => setName(e.target.value)}
              />
              <button
                type="submit"
                className="button"
                disabled={busy || !name.trim()}
              >
                Create workspace
              </button>
            </form>
          </section>
          <section>
            <h2>Open workspace</h2>
            <p>
              Use an existing directory, including one that has no files.
              Previous local notes and results are restored when available.
            </p>
            <button
              type="button"
              className="button secondary"
              disabled={busy}
              onClick={() => void open({ intent: "open" })}
            >
              Choose local directory
            </button>
          </section>
        </div>
        <p role="status" aria-live="polite">
          {status}
        </p>
        {error && <p role="alert">{error}</p>}
        <section aria-label="Recent workspaces">
          <div className="launcher-recents-heading">
            <h2>Recent workspaces</h2>
            <button
              type="button"
              className="button secondary"
              disabled={busy}
              onClick={() => {
                setError(null);
                setGeneration((v) => v + 1);
              }}
            >
              Refresh recent workspaces
            </button>
          </div>
          {loaded?.generation !== generation ? (
            <p>
              {error
                ? "Recent workspaces are unavailable."
                : "Loading recent workspaces…"}
            </p>
          ) : items.length === 0 ? (
            <p>No local workspaces are registered yet.</p>
          ) : (
            <ul className="launcher-recents">
              {items.map((item) => (
                <li key={item.id}>
                  <button
                    type="button"
                    disabled={busy || !item.available}
                    onClick={() => void open({ intent: "recent", id: item.id })}
                  >
                    <strong>{item.name}</strong>
                    <span className="launcher-recent-status">
                      {item.available
                        ? "Resume local investigation"
                        : "Source directory unavailable — restore it, then refresh"}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      </main>
    </div>
  );
}
