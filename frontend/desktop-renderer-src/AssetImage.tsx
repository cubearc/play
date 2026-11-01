import { useEffect, useState } from "react";
import { parseJson, stringifyJson } from "../desktop/json-codec";
import {
  type AssetView,
  type Context,
  type ImageRead,
  imageRead,
} from "../desktop/workspace-contract";
export function AssetImage({
  asset,
  context,
}: {
  asset: AssetView;
  context: Context;
}) {
  return (
    <ImagePreview
      context={context}
      source={{ kind: "asset", asset_id: asset.id, revision: asset.revision }}
      title={asset.title}
    />
  );
}
export function ImagePreview({
  context,
  source,
  title,
}: ImageRead & { title: string }) {
  const [url, setUrl] = useState("");
  const [failed, setFailed] = useState(false);
  const reference = stringifyJson({ context, source });
  useEffect(() => {
    let current = true;
    setUrl("");
    setFailed(false);
    void window.play
      .image(imageRead(parseJson(reference)))
      .then((url) => {
        if (current) setUrl(url);
      })
      .catch(() => {
        if (current) setFailed(true);
      });
    return () => {
      current = false;
    };
  }, [reference]);
  if (failed)
    return (
      <span className="image-unavailable">
        Image unavailable. Reopen this view to request a fresh preview.
      </span>
    );
  return url ? (
    <img
      className="asset-image"
      src={url}
      alt={title}
      onError={() => setFailed(true)}
    />
  ) : (
    <span className="image-loading">Loading image…</span>
  );
}
