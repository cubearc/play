import { useEffect, useState } from "react";
import { apiFetch } from "./capability";
import { object } from "./lab-api";

type Report = {
  status: "inputs_available" | "gaps";
  checked_artifacts: number;
  gaps: {
    artifact_id: string;
    input_port: string | null;
    code: string;
    message: string;
  }[];
};
function parse(value: unknown, id: string): Report {
  const v = object(value);
  if (
    v.schema_version !== 1 ||
    v.artifact_id !== id ||
    (v.status !== "inputs_available" && v.status !== "gaps") ||
    typeof v.checked_artifacts !== "number" ||
    !Number.isSafeInteger(v.checked_artifacts) ||
    v.checked_artifacts < 0 ||
    v.checked_artifacts > 64 ||
    !Array.isArray(v.gaps) ||
    v.gaps.length > 256 ||
    (v.status === "inputs_available" && v.gaps.length > 0)
  )
    throw new Error("The replay check returned an incompatible report.");
  return {
    status: v.status,
    checked_artifacts: v.checked_artifacts,
    gaps: v.gaps.map((item) => {
      const gap = object(item);
      if (
        typeof gap.code !== "string" ||
        gap.code.length > 128 ||
        typeof gap.artifact_id !== "string" ||
        gap.artifact_id.length > 128 ||
        (gap.input_port !== null &&
          (typeof gap.input_port !== "string" ||
            gap.input_port.length > 128)) ||
        typeof gap.message !== "string" ||
        gap.message.length > 1024
      )
        throw new Error("The replay check returned an incompatible gap.");
      return {
        artifact_id: gap.artifact_id,
        code: gap.code,
        input_port: gap.input_port as string | null,
        message: gap.message,
      };
    }),
  };
}
export function ReplayStatus({ id }: { id: string }) {
  const [generation, setGeneration] = useState(0);
  const [loaded, setLoaded] = useState<{
    id: string;
    generation: number;
    report: Report;
  } | null>(null);
  const value =
    loaded?.id === id && loaded.generation === generation
      ? loaded.report
      : null;
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    setError(null);
    void (async () => {
      const response = await apiFetch(
        `/api/lab/results/${encodeURIComponent(id)}/replay`,
        { signal: controller.signal },
      );
      if (!response.ok)
        throw new Error(
          "Replay inputs could not be checked. The result is still available.",
        );
      const report = parse(await response.json(), id);
      if (!controller.signal.aborted) setLoaded({ id, generation, report });
    })().catch((error: unknown) => {
      if (!controller.signal.aborted)
        setError(
          error instanceof Error
            ? error.message
            : "Replay inputs could not be checked.",
        );
    });
    return () => controller.abort();
  }, [id, generation]);
  return (
    <section aria-label="Replay availability">
      <h4>Replay inputs</h4>
      {error ? (
        <p role="alert">{error}</p>
      ) : !value ? (
        <p role="status">Checking retained evidence and authorized inputs…</p>
      ) : (
        <>
          <p role="status">
            {value.status === "inputs_available"
              ? "Recorded inputs are currently available."
              : "Replay has missing or changed evidence."}{" "}
            Checked {value.checked_artifacts} retained artifacts.
          </p>
          {value.gaps.length > 0 && (
            <ul>
              {value.gaps.map((gap) => (
                <li key={`${gap.artifact_id}:${gap.input_port}:${gap.code}`}>
                  {gap.input_port && <strong>{gap.input_port}: </strong>}
                  {gap.message}
                </li>
              ))}
            </ul>
          )}
        </>
      )}
      <p>
        This bounded check does not execute operations, copy sources, verify
        answers or guarantee an identical rerun.
      </p>
      <button
        type="button"
        className="button secondary"
        onClick={() => setGeneration((v) => v + 1)}
      >
        Check replay inputs again
      </button>
    </section>
  );
}
