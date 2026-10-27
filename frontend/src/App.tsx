import { useEffect, useState } from "react";
import {
  connect,
  type Health,
  type LaunchLink,
  SessionError,
  takeLaunchLink,
} from "./api";
import { sessionToken } from "./capability";
import { chooserOrigin } from "./chooser-origin";
import { InvestigationWorkspace } from "./InvestigationWorkspace";

type Connection =
  | { state: "connecting" }
  | { state: "ready"; health: Health }
  | { state: "error"; message: string };

function CubeMark({ large = false }: { large?: boolean }) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      className={large ? "cube-mark large" : "cube-mark"}
    >
      <path
        d="m24 4 18 10v20L24 44 6 34V14L24 4Z"
        stroke="currentColor"
        strokeWidth="2"
      />
      <path
        d="m6 14 18 10 18-10M24 24v20M15 9l18 10v20"
        stroke="currentColor"
        strokeWidth="2"
      />
    </svg>
  );
}

export default function App() {
  const [connectionRequest, setConnectionRequest] = useState(takeLaunchLink);
  const [connection, setConnection] = useState<Connection>({
    state: "connecting",
  });
  const [pendingLaunch, setPendingLaunch] = useState<LaunchLink | null>(null);
  useEffect(() => {
    const receive = () => {
      const link = takeLaunchLink();
      if (!link.session && !link.selection) return;
      if (connection.state !== "ready") setConnectionRequest(link);
      else if (link.selection || link.session !== sessionToken())
        setPendingLaunch(link);
    };
    window.addEventListener("hashchange", receive);
    return () => window.removeEventListener("hashchange", receive);
  }, [connection.state]);

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
  const chooser = chooserOrigin();
  return (
    <div className="app-shell">
      <header className="app-header">
        <a href="/" className="brand" aria-label="Cube Play home">
          <CubeMark />
          <span>
            Cube <span className="brand-light">Play</span>
          </span>
        </a>
        <div className="header-actions">
          {chooser && (
            <a className="button secondary" href={`${chooser}/`}>
              Workspaces
            </a>
          )}
          <span className="privacy-badge">
            <span className="privacy-icon" aria-hidden="true">
              ⌁
            </span>{" "}
            Local by design
          </span>
        </div>
      </header>
      <main
        id="main-content"
        className="workspace mx-auto w-full max-w-6xl px-6 py-6 sm:px-8 sm:py-8"
      >
        {pendingLaunch && (
          <section aria-label="Received launcher link">
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
        <div className="workspace-heading">
          <div>
            <p className="eyebrow">YOUR LOCAL LABORATORY</p>
            <h1>
              {connection.state === "ready" && connection.health.workspace
                ? connection.health.workspace.name
                : "Investigation workspace"}
            </h1>
          </div>
          <span className={`connection-badge ${ready ? "connected" : ""}`}>
            <span className="status-dot" />
            {ready
              ? "Connected locally"
              : connection.state === "error"
                ? "Connection unavailable"
                : "Connecting"}
          </span>
        </div>
        {connection.state === "ready" && connection.health.workspace ? (
          <section
            className="session-panel workspace-context"
            aria-label="Open workspace"
          >
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
            <div className="illustration" aria-hidden="true">
              <div className="orbit orbit-one" />
              <div className="orbit orbit-two" />
              <div className="cube-tile">
                <CubeMark large />
              </div>
              <span className="coordinate coordinate-top">01 / LOCAL</span>
              <span className="coordinate coordinate-bottom">CUBE · PLAY</span>
            </div>
            <div className="welcome-copy">
              <span className="section-label">
                A SPACE TO FOLLOW THE EVIDENCE
              </span>
              <h2 id="welcome-title">
                Stay curious.
                <br />
                <span>Keep it local.</span>
              </h2>
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
          <span>Private files. Local execution. Your investigation.</span>
          <span>POWERED BY CUBE</span>
        </footer>
      </main>
    </div>
  );
}
