import { lazy, Suspense, useState } from "react";
import type { Json, OperationResult } from "./exploration-api";
import { histogram } from "./lab-api";
import { ReplayStatus } from "./ReplayStatus";
import { SavedResultActions } from "./SavedResultActions";
import { TypedImage } from "./TypedImage";

const LabHistogram = lazy(() => import("./LabHistogram"));
const PREVIEW_LIMIT = 65536;

// A closed set of Play-owned viewers. Contract content cannot load components.
function viewer(
  semantic: string,
): "text" | "bytes" | "histogram" | "image" | "structured" {
  if (semantic === "image.samples") return "image";
  if (semantic === "text" || semantic.startsWith("text.")) return "text";
  if (semantic === "bytes" || semantic === "byte_reader") return "bytes";
  if (semantic === "image.histogram") return "histogram";
  return "structured";
}
export function TypedValue({
  semantic,
  value,
}: {
  semantic: string;
  value: Json;
}) {
  const kind = viewer(semantic);
  if (kind === "image") return <TypedImage value={value} />;
  if (kind === "text" && typeof value === "string")
    return (
      <>
        {value === "" && <p>The result is empty text.</p>}
        <textarea
          className="text-preview"
          readOnly
          rows={6}
          aria-label="Exact text output"
          value={value.slice(0, PREVIEW_LIMIT)}
        />
      </>
    );
  if (
    kind === "bytes" &&
    value &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    typeof value.base64 === "string"
  )
    return (
      <>
        <p>Exact bytes in Base64 transport. No text decoding is inferred.</p>
        <textarea
          className="text-preview"
          readOnly
          rows={6}
          aria-label="Base64 byte output"
          value={value.base64.slice(0, PREVIEW_LIMIT)}
        />
      </>
    );
  if (kind === "histogram") {
    try {
      const data = histogram(value);
      return (
        <Suspense fallback={<p role="status">Loading histogram…</p>}>
          <LabHistogram data={data} />
        </Suspense>
      );
    } catch {
      // Unknown precision/shape remains inspectable without inventing counts.
    }
  }
  return (
    <textarea
      className="text-preview"
      readOnly
      rows={8}
      aria-label="Structured output"
      value={JSON.stringify(value, null, 2).slice(0, PREVIEW_LIMIT)}
    />
  );
}
export function TypedOutputs({
  result,
  initialPort,
  onAnnotate,
  onPreserve,
}: {
  result: OperationResult;
  initialPort?: string | undefined;
  onAnnotate?: ((port: string) => void) | undefined;
  onPreserve?: ((port: string) => void) | undefined;
}) {
  const [selected, setSelected] = useState(
    initialPort ?? result.ports[0]?.name ?? "",
  );
  const port = result.ports.find((port) => port.name === selected);
  const value = result.output[selected];
  return (
    <section aria-label="Selected operation outputs">
      <h4>Result · {String(result.record.provenance.operation)}</h4>
      <label>
        Output port
        <select
          value={selected}
          onChange={(event) => setSelected(event.target.value)}
        >
          {result.ports.map((port) => (
            <option key={port.name} value={port.name}>
              {port.name} · {port.semantic_type}
            </option>
          ))}
        </select>
      </label>
      {port && (
        <p>
          Operation version {port.operation_version} · schema{" "}
          {port.schema_version} · artifact <code>{result.id}</code> /{" "}
          <code>{port.name}</code>
        </p>
      )}
      {port && value !== undefined ? (
        <TypedValue
          key={port.name}
          semantic={port.semantic_type}
          value={value}
        />
      ) : (
        <p>This optional output was not produced.</p>
      )}
      {onAnnotate && result.record.retained && port && value !== undefined && (
        <button
          type="button"
          className="button secondary"
          onClick={() => onAnnotate(port.name)}
        >
          Annotate selected output
        </button>
      )}
      {onPreserve && result.record.retained && port && value !== undefined && (
        <button
          type="button"
          className="button secondary"
          onClick={() => onPreserve(port.name)}
        >
          Preserve as solution
        </button>
      )}
      <p>
        Preview is limited to 65,536 characters. Export retains the complete
        output bundle.
      </p>
      <details>
        <summary>Output data and provenance</summary>
        <textarea
          className="text-preview"
          readOnly
          rows={8}
          aria-label="Named output bundle JSON"
          value={JSON.stringify(result.output, null, 2).slice(0, PREVIEW_LIMIT)}
        />
        <textarea
          className="text-preview"
          readOnly
          rows={8}
          aria-label="Recorded operation provenance JSON"
          value={JSON.stringify(result.record.provenance, null, 2).slice(
            0,
            PREVIEW_LIMIT,
          )}
        />
        {port && (
          <p>
            Schema SHA-256: <code>{port.schema_sha256}</code>
          </p>
        )}
      </details>
      {result.record.retained && <SavedResultActions result={result.record} />}
      {result.record.retained && (
        <ReplayStatus key={result.id} id={result.id} />
      )}
    </section>
  );
}
