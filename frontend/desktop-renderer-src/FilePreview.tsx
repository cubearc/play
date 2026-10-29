import type { FileContent } from "../desktop/workspace-contract";

export function FilePreview({
  content,
  details = false,
}: {
  content: FileContent;
  details?: boolean;
}) {
  return (
    <div className="file-preview">
      <span className="file-meta">
        {content.byteLength.toLocaleString("en-US")} bytes ·{" "}
        {content.preview.format === "utf8" ? "UTF-8 preview" : "Hex preview"}
      </span>
      <pre>{content.preview.text || "Empty file"}</pre>
      {content.preview.truncated ? (
        <span className="file-meta">
          Partial preview · original bytes are preserved
        </span>
      ) : null}
      {details ? (
        <div className="file-digest">
          <span>SHA-256</span>
          <code>{content.sha256}</code>
        </div>
      ) : null}
    </div>
  );
}
