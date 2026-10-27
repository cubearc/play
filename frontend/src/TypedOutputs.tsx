import { lazy, Suspense, useState } from "react";
import type { Json, OperationResult } from "./exploration-api";
import { histogram } from "./lab-api";
import { SavedResultActions } from "./SavedResultActions";

const LabHistogram = lazy(() => import("./LabHistogram"));
const PREVIEW_LIMIT = 65536;

// A closed set of Play-owned viewers. Contract content cannot load components.
function viewer(
  semantic: string,
): "text" | "bytes" | "histogram" | "structured" {
  if (semantic === "text" || semantic.startsWith("text.")) return "text";
  if (semantic === "bytes" || semantic === "byte_reader") return "bytes";
  if (semantic === "image.histogram") return "histogram";
  return "structured";
}
function Content({ semantic, value }: { semantic: string; value: Json }) {
  const kind = viewer(semantic);
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
export function TypedOutputs({ result }: { result: OperationResult }) {
  const [selected, setSelected] = useState(result.ports[0]?.name ?? "");
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
        <Content key={port.name} semantic={port.semantic_type} value={value} />
      ) : (
        <p>This optional output was not produced.</p>
      )}
      <p>
        Preview is limited to 65,536 characters. Export retains the complete
        output bundle.
      </p>
      <details>
        <summary>Output data and provenance</summary>
        <pre>
          {JSON.stringify(result.output, null, 2).slice(0, PREVIEW_LIMIT)}
        </pre>
        <pre>
          {JSON.stringify(result.record.provenance, null, 2).slice(
            0,
            PREVIEW_LIMIT,
          )}
        </pre>
        {port && (
          <p>
            Schema SHA-256: <code>{port.schema_sha256}</code>
          </p>
        )}
      </details>
      {result.record.retained && <SavedResultActions result={result.record} />}
    </section>
  );
}
