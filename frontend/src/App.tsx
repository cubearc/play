import { useEffect, useEffectEvent, useState } from "react";
import {
  connect,
  type Health,
  type LaunchLink,
  SessionError,
  takeLaunchLink,
} from "./api";
import { sessionToken } from "./capability";
import { chooserOrigin } from "./chooser-origin";
import { Brand } from "./InterfaceIcon";
import { InvestigationWorkspace } from "./InvestigationWorkspace";

type Connection =
  | { state: "connecting" }
  | { state: "ready"; health: Health }
  | { state: "error"; message: string };

export default function App() {
  const [connectionRequest, setConnectionRequest] = useState(takeLaunchLink);
  const [connection, setConnection] = useState<Connection>({
    state: "connecting",
  });
  const [pendingLaunch, setPendingLaunch] = useState<LaunchLink | null>(null);
  // A committed ready state must govern links even before passive effects run.
  // Keep the listener installed while reading the latest connection state.
  const receive = useEffectEvent(() => {
    const link = takeLaunchLink();
    if (!link.session && !link.selection) return;
    if (connection.state !== "ready") setConnectionRequest(link);
    else if (link.selection || link.session !== sessionToken())
      setPendingLaunch(link);
  });
  useEffect(() => {
    window.addEventListener("hashchange", receive);
    return () => window.removeEventListener("hashchange", receive);
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    setConnection({ state: "connecting" });
    connect(connectionRequest.session, controller.signal).then(
      (health) => {
        if (!controller.signal.aborted)
          setConnection({ state: "ready", health });
      },
      (error: unknown) => {
        if (!controller.signal.aborted)
          setConnection({
            state: "error",
            message:
              error instanceof SessionError
                ? error.message
                : "Could not reach Cube Play. Check that the local application is running, then try again.",
          });
      },
    );
    return () => controller.abort();
  }, [connectionRequest]);

  const ready = connection.state === "ready";
  const workspace = ready ? connection.health.workspace : undefined;
  const chooser = chooserOrigin();
  return (
    <div className={`app-shell${workspace ? " workbench-shell" : ""}`}>
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      <header className="app-header">
        <a href="/" className="brand" aria-label="CubePlay home">
          <Brand />
        </a>
        {workspace && (
          <div className="header-workspace">
            <span aria-hidden="true">/</span>
            <h1>{workspace.name}</h1>
          </div>
        )}
        <div className="header-actions">
          {chooser && (
            <a className="button secondary" href={`${chooser}/`}>
              Workspaces
            </a>
          )}
          <span className={`connection-badge ${ready ? "connected" : ""}`}>
            <span className="status-dot" />
            {ready
              ? "Connected locally"
              : connection.state === "error"
                ? "Connection unavailable"
                : "Connecting"}
          </span>
        </div>
      </header>
      <main id="main-content" className="workspace">
        {pendingLaunch && (
          <section
            className="pending-launch"
            aria-label="Received launcher link"
          >
            <p>
              A new launcher link is ready. Save any edits before opening it.
            </p>
            <button
              className="button secondary"
              type="button"
              onClick={() => {
                setConnectionRequest(pendingLaunch);
                setPendingLaunch(null);
              }}
            >
              Open received session
            </button>
          </section>
        )}
        {!workspace && (
          <div className="workspace-heading">
            <div>
              <p className="eyebrow">CUBE PLAY / CONNECTION</p>
              <h1>Investigation workspace</h1>
            </div>
          </div>
        )}
        {connection.state === "ready" && connection.health.workspace ? (
          <section className="workspace-context" aria-label="Open workspace">
            <div>
              <p>
                <strong>Source files are read-only.</strong> Notes and results
                stay local.
              </p>
              {connectionRequest.selection ? (
                <p className="selected-file">
                  <strong>
                    {connectionRequest.selection.intent === "challenge"
                      ? "Opened challenge"
                      : "Opened file"}
                    :
                  </strong>{" "}
                  {connectionRequest.selection.name}
                </p>
              ) : null}
            </div>
          </section>
        ) : (
          <section className="welcome-panel" aria-labelledby="welcome-title">
            <div className="welcome-copy">
              <h2 id="welcome-title">Your files. Your workspace.</h2>
              <p>
                A private workspace for investigating files, exploring ideas,
                and keeping discoveries together.
              </p>
            </div>
          </section>
        )}
        {connection.state === "ready" && connection.health.workspace && (
          <InvestigationWorkspace
            initialSelection={connectionRequest.selection}
          />
        )}
        <details
          className="connection-details"
          open={connection.state !== "ready" || !connection.health.workspace}
        >
          <summary>Local connection</summary>
          <section className="session-panel" aria-labelledby="session-title">
            <div>
              <p className="eyebrow">LOCAL SESSION</p>
              <h2 id="session-title">
                {ready
                  ? "Your connection is ready"
                  : connection.state === "error"
                    ? "Let's reconnect"
                    : "Connecting to your application"}
              </h2>
              <p role="status" aria-live="polite">
                {connection.state === "ready"
                  ? "Connected to Cube Play on this machine. No account or cloud connection is required."
                  : connection.state === "error"
                    ? connection.message
                    : "Checking the local application…"}
              </p>
            </div>
            {connection.state === "error" ? (
              <button
                type="button"
                className="button"
                onClick={() =>
                  setConnectionRequest((request) => ({ ...request }))
                }
              >
                Try again <span aria-hidden="true">↗</span>
              </button>
            ) : (
              <span className="session-detail">
                {connection.state === "ready"
                  ? `Cube Play ${connection.health.version}`
                  : "Please wait"}
              </span>
            )}
          </section>
        </details>
        <footer className="workspace-footer">
          <span>Files stay on this device.</span>
          <span>POWERED BY CUBE</span>
        </footer>
      </main>
    </div>
  );
}
