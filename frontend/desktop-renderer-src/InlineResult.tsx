import type { Attempt } from "./useExecutions";

function value(attempt: Attempt): string {
  if (attempt.running) return "Running…";
  if (attempt.error) return attempt.error;
  const result = attempt.result;
  if (!result) return "No result.";
  if (result.status === "cancelled") return "Cancelled";
  if (result.status === "failed") return `Execution failed: ${result.code}`;
  return "text" in result.output
    ? result.output.text
    : JSON.stringify(result.output.bytes);
}

export function InlineResult({ attempts }: { attempts: readonly Attempt[] }) {
  const latest = attempts.at(-1);
  if (!latest) return null;
  return (
    <section className="inline-result" aria-label="Cell result">
      <div className="cell-result-label">
        <span>Out [{attempts.length}]</span>
        <span>
          {latest.running ? "Running" : `${latest.elapsed} ms`} · Core v
          {latest.operation.operation_version}
        </span>
      </div>
      <pre role="status">{value(latest)}</pre>
      <details>
        <summary>
          Input reference & session attempts ({attempts.length})
        </summary>
        <code>
          {latest.assetId} @ revision {latest.assetRevision}
        </code>
        <ol>
          {attempts.map((attempt) => (
            <li key={attempt.id} title={attempt.operation.operation}>
              {attempt.operation.name} ·{" "}
              {attempt.running
                ? "Running"
                : (attempt.result?.status ?? "Rejected")}{" "}
              · {attempt.elapsed} ms
            </li>
          ))}
        </ol>
      </details>
    </section>
  );
}
