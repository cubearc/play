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
      {details ? (
        <textarea
          className="file-source-preview"
          aria-label="File preview"
          readOnly
          value={content.preview.text}
          placeholder="Empty file"
          rows={6}
          spellCheck={false}
        />
      ) : (
        <pre>{content.preview.text || "Empty file"}</pre>
      )}
      {content.preview.truncated ? (
        <span className="file-meta">
          Partial preview · original bytes are preserved
        </span>
      ) : null}
      {details ? (
        <div className="file-digest">
          <span>SHA-256</span>
          <input
            aria-label="File SHA-256"
            readOnly
            value={content.sha256}
            spellCheck={false}
          />
        </div>
      ) : null}
    </div>
  );
}
