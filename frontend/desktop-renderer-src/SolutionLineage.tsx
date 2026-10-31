import type { SolutionPath, SolutionView } from "../desktop/workspace-contract";
import VscodeButton from "./VscodeButton";

export function ContributingPath({ path }: { path: SolutionPath }) {
  return (
    <>
      <ol>
        {path.nodes.map((node) => (
          <li key={node.id}>
            {node.name} · {node.ports.join(", ")}
          </li>
        ))}
      </ol>
      <p>
        Pipeline r{path.pipeline.revision} · Trail r{path.trail.revision}
      </p>
      {path.trail.hypothesis ? (
        <p className="solution-explanation">{path.trail.hypothesis}</p>
      ) : null}
    </>
  );
}
export function SolutionLineage({
  lineage,
  openRun,
}: {
  lineage: SolutionView["snapshot"]["lineage"];
  openRun: (origin: { trailId: string; runId: string }) => void;
}) {
  if (!lineage)
    return (
      <p className="muted">
        This saved revision does not include source history.
      </p>
    );
  if (!lineage.assets.length && !lineage.paths.length) return null;
  return (
    <section aria-label="Recorded source history">
      <h4>Recorded source history</h4>
      <ul>
        {lineage.assets.map((asset) => (
          <li key={`${asset.assetId}:${asset.revision}`}>
            {asset.title} · r{asset.revision}
            {asset.origin?.kind === "asset" ? (
              <span className="muted">
                {" "}
                · Copied from {asset.origin.title} r{asset.origin.revision}
                {asset.editedSinceCopy
                  ? " · Edited after copying; earlier Runs describe its source, not these edited bytes."
                  : ""}
              </span>
            ) : null}
            {asset.origin?.kind === "run_output" ? (
              <span className="muted">
                {" "}
                · Saved from Run {asset.origin.sequence}, {asset.origin.port}
              </span>
            ) : null}
          </li>
        ))}
      </ul>
      {lineage.paths.map(({ selection, path }) => {
        const origin = lineage.assets.find(
          (asset) =>
            asset.origin?.kind === "run_output" &&
            selection.kind === "run_output" &&
            asset.origin.runId === selection.run_id &&
            asset.origin.attemptId === selection.attempt_id &&
            asset.origin.port === selection.port,
        )?.origin;
        return (
          <div key={JSON.stringify(selection)}>
            <h5>
              {origin?.kind === "run_output"
                ? `Run ${origin.sequence} · ${origin.operationName} · ${origin.port}`
                : "Contributing Run"}
            </h5>
            <ContributingPath path={path} />
            <VscodeButton
              secondary
              onClick={() =>
                openRun({ trailId: path.trail.id, runId: path.runId })
              }
            >
              Open contributing Run
            </VscodeButton>
          </div>
        );
      })}
    </section>
  );
}
