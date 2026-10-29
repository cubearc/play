import type { AssetView, RunOutputOrigin } from "../desktop/workspace-contract";
import VscodeButton from "./VscodeButton";

export function AssetOrigin({
  asset,
  openRun,
}: {
  asset: AssetView;
  openRun: (origin: RunOutputOrigin) => void;
}) {
  const origin = asset.origin;
  if (!origin) return null;
  if (origin.kind === "asset")
    return (
      <p className="copy-origin">
        Copied from {origin.title} · r{origin.revision}
      </p>
    );
  return (
    <div className="result-origin">
      <span>
        Saved from {origin.operationName} · {origin.port} · Run{" "}
        {origin.sequence}
      </span>
      <VscodeButton
        secondary
        icon="history"
        aria-label={`Open source Run ${origin.sequence}`}
        onClick={() => openRun(origin)}
      >
        Open source Run
      </VscodeButton>
    </div>
  );
}
