import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Operation, RunResult } from "../desktop/contract";
import "./style.css";

declare global {
  interface Window {
    play: {
      catalog: () => Promise<Operation[]>;
      run: (
        operation: string,
        version: number,
        input: string,
      ) => Promise<RunResult>;
      cancel: () => Promise<void>;
    };
  }
}

function App() {
  const [operation, setOperation] = useState("");
  const [operations, setOperations] = useState<Operation[]>([]);
  const [input, setInput] = useState("Hello, CubeArc!");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState("Ready");
  useEffect(() => {
    void window.play
      .catalog()
      .then((entries) => {
        setOperations(entries);
        setOperation(entries[0]?.operation ?? "");
      })
      .catch(() => setResult("Engine catalog unavailable."));
  }, []);
  async function run() {
    setRunning(true);
    setResult("Running");
    try {
      const selected = operations.find(
        (entry) => entry.operation === operation,
      );
      if (!selected) throw new Error("Operation unavailable");
      const result = await window.play.run(
        operation,
        selected.operation_version,
        input,
      );
      setResult(JSON.stringify(result, null, 2));
    } catch {
      setResult("Execution rejected. Check the native proof requirements.");
    } finally {
      setRunning(false);
    }
  }
  return (
    <div className="ide">
      <header>
        <strong>Cube Play</strong>
        <span>Native boundary proof</span>
      </header>
      <aside>
        <small>DESKTOP ENGINE</small>
        <p>Private pipes</p>
        <p>Core workers</p>
        <p>macOS App Sandbox</p>
      </aside>
      <main>
        <div className="tab">Core invocation</div>
        <section>
          <h1>Electron → Rust → Cube</h1>
          <p>
            This development proof uses ephemeral text. Workspace editing is
            implemented in the following refactor deliveries.
          </p>
          <label htmlFor="operation">Operation</label>
          <select
            id="operation"
            value={operation}
            disabled={running}
            onChange={(event) => setOperation(event.target.value)}
          >
            {operations.map((entry) => (
              <option key={entry.operation} value={entry.operation}>
                {entry.description}
              </option>
            ))}
          </select>
          <label htmlFor="input">Input</label>
          <textarea
            id="input"
            value={input}
            disabled={running}
            onChange={(event) => setInput(event.target.value)}
          />
          <div className="actions">
            <button
              type="button"
              disabled={running || !operation}
              onClick={() => void run()}
            >
              Run
            </button>
            <button
              type="button"
              disabled={!running}
              onClick={() => void window.play.cancel().catch(() => {})}
            >
              Cancel
            </button>
          </div>
          <h2>Engine response</h2>
          <pre role="status">{result}</pre>
        </section>
      </main>
      <footer>Protocol 1 · Ephemeral execution · No workspace loaded</footer>
    </div>
  );
}

const root = document.getElementById("root");
if (root) createRoot(root).render(<App />);
