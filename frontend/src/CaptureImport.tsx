import { useEffect, useRef, useState } from "react";
import { importCapture, type OperationResult } from "./exploration-api";
import { TypedOutputs } from "./TypedOutputs";
import type { Store } from "./workspace-api";

export function CaptureImport({
  asset,
}: {
  asset: { store: Store; path: string };
}) {
  const [result, setResult] = useState<OperationResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const active = useRef<AbortController | null>(null);
  useEffect(() => () => active.current?.abort(), []);
  async function publish() {
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setError(null);
    try {
      setResult(
        await importCapture(asset.store, asset.path, controller.signal),
      );
    } catch (error) {
      if (!controller.signal.aborted)
        setError(
          error instanceof Error
            ? error.message
            : "The selected CLI capture could not be imported.",
        );
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  return (
    <details>
      <summary>Import a selected CLI capture</summary>
      <p>
        Selected file: {asset.path}. Import preserves exact named outputs
        without running a command. A capture file declares its provenance; it
        does not prove execution or authorize replay inputs.
      </p>
      <button
        type="button"
        className="button secondary"
        disabled={busy}
        onClick={() => void publish()}
      >
        {busy ? "Importing capture…" : "Import selected CLI capture"}
      </button>
      {error && <p role="alert">{error}</p>}
      {result && <TypedOutputs key={result.id} result={result} />}
    </details>
  );
}
