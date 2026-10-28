import { useEffect, useState } from "react";
import type { AssetView, Context } from "../desktop/workspace-contract";
export function AssetImage({
  asset,
  context,
}: {
  asset: AssetView;
  context: Context;
}) {
  const [url, setUrl] = useState("");
  const [failed, setFailed] = useState(false);
  const { workspace_id, exploration_id, stage_id } = context;
  useEffect(() => {
    let current = true;
    setUrl("");
    setFailed(false);
    void window.play
      .image({
        context: { workspace_id, exploration_id, stage_id },
        asset_id: asset.id,
        revision: asset.revision,
      })
      .then((url) => {
        if (current) setUrl(url);
      })
      .catch(() => {
        if (current) setFailed(true);
      });
    return () => {
      current = false;
    };
  }, [asset.id, asset.revision, workspace_id, exploration_id, stage_id]);
  if (failed)
    return (
      <span className="image-unavailable">
        Image unavailable. Reopen the asset canvas to request a fresh preview.
      </span>
    );
  return url ? (
    <img
      className="asset-image"
      src={url}
      alt={asset.title}
      onError={() => setFailed(true)}
    />
  ) : (
    <span className="image-loading">Loading image…</span>
  );
}
