import { useEffect, useRef, useState } from "react";
import { Brand, InterfaceIcon } from "./InterfaceIcon";
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
  const [naming, setNaming] = useState(false);
  const nameInput = useRef<HTMLInputElement | null>(null);
  const newButton = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    if (naming) nameInput.current?.focus();
  }, [naming]);
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
    <div className="app-shell launcher-shell">
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      <header className="app-header">
        <span className="brand">
          <Brand />
        </span>
        <span className="privacy-badge">
          <span className="status-dot" /> Local workspace
        </span>
      </header>
      <main id="main-content" className="launcher-home">
        <div className="launcher-heading">
          <div>
            <p className="eyebrow">CUBE PLAY / WORKSPACES</p>
            <h1>Your workspaces</h1>
            <p className="launcher-subtitle">
              Pick up an investigation, or start with something new.
            </p>
          </div>
          <div className="launcher-actions">
            <button
              ref={newButton}
              type="button"
              className="button"
              disabled={busy}
              aria-expanded={naming}
              aria-controls="new-workspace"
              onClick={() => setNaming(true)}
            >
              <InterfaceIcon name="plus" /> New workspace
            </button>
            <button
              type="button"
              className="button secondary"
              disabled={busy}
              onClick={() => void open({ intent: "open" })}
            >
              <InterfaceIcon name="files" /> Open folder
            </button>
          </div>
        </div>
        {naming && (
          <section
            id="new-workspace"
            className="launcher-create"
            aria-labelledby="new-workspace-title"
          >
            <div>
              <h2 id="new-workspace-title">Create a workspace</h2>
              <p>Name your folder. Next, choose where to create it.</p>
            </div>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void open({ intent: "new", name });
              }}
            >
              <div className="launcher-name-field">
                <label htmlFor="workspace-name">Workspace folder name</label>
                <input
                  id="workspace-name"
                  ref={nameInput}
                  value={name}
                  maxLength={128}
                  required
                  disabled={busy}
                  onChange={(e) => setName(e.target.value)}
                />
              </div>
              <button
                type="submit"
                className="button"
                disabled={busy || !name.trim()}
              >
                {busy ? "Opening folder selection…" : "Create workspace"}
              </button>
              <button
                type="button"
                className="button ghost"
                disabled={busy}
                onClick={() => {
                  setNaming(false);
                  newButton.current?.focus();
                }}
              >
                Cancel
              </button>
            </form>
          </section>
        )}
        <p
          className={`launcher-status${busy ? " is-busy" : ""}`}
          role="status"
          aria-live="polite"
        >
          {status}
        </p>
        {error && (
          <p className="interface-error" role="alert">
            {error}
          </p>
        )}
        <section
          className="launcher-recent-panel"
          aria-label="Recent workspaces"
          aria-busy={busy}
        >
          <div className="launcher-recents-heading">
            <h2>
              Recent workspaces{" "}
              {loaded?.generation === generation && (
                <span className="item-count">{items.length}</span>
              )}
            </h2>
            <button
              type="button"
              className="button ghost"
              aria-label="Refresh recent workspaces"
              disabled={busy}
              onClick={() => {
                setError(null);
                setGeneration((v) => v + 1);
              }}
            >
              <InterfaceIcon name="refresh" /> Refresh
            </button>
          </div>
          {loaded?.generation !== generation ? (
            <p className="launcher-empty">
              {error
                ? "Recent workspaces are unavailable."
                : "Loading recent workspaces…"}
            </p>
          ) : items.length === 0 ? (
            <div className="launcher-empty">
              <span className="empty-folder">
                <InterfaceIcon name="files" />
              </span>
              <h3>A fresh place to start</h3>
              <p>No local workspaces are registered yet.</p>
              <p>Create a workspace or open a folder to begin.</p>
            </div>
          ) : (
            <ul className="launcher-recents">
              {items.map((item) => (
                <li key={item.id}>
                  <button
                    type="button"
                    disabled={busy || !item.available}
                    onClick={() => void open({ intent: "recent", id: item.id })}
                  >
                    <span className="recent-folder">
                      <InterfaceIcon name="files" />
                    </span>
                    <span className="launcher-recent-copy">
                      <strong>{item.name}</strong>
                      <span className="launcher-recent-status">
                        {item.available
                          ? "Resume local investigation"
                          : "Source directory unavailable — restore it, then refresh"}
                      </span>
                    </span>
                    <span className="recent-arrow">
                      <InterfaceIcon name="arrow" />
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
        <footer className="launcher-footer">
          <span>
            <span className="status-dot" /> Files stay on this device.
          </span>
          <span>
            Source files are read-only. Notes and results are saved separately.
          </span>
        </footer>
      </main>
    </div>
  );
}
