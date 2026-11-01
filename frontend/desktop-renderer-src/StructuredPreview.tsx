import type { StructuredContent } from "../desktop/workspace-contract";

/** A bounded notebook preview never substitutes for the exact typed input. */
export function StructuredPreview({
  content,
  details = false,
}: {
  content: StructuredContent;
  details?: boolean;
}) {
  const serialized = JSON.stringify(content.value, null, 2);
  const preview = serialized.slice(0, 1024);
  return (
    <div className="file-preview">
      <span className="file-meta">
        {content.semanticType} · Structured data
      </span>
      {details ? (
        <textarea
          className="file-source-preview"
          aria-label="Structured data"
          readOnly
          value={serialized}
          rows={Math.min(8, Math.max(2, serialized.split("\n").length))}
          spellCheck={false}
        />
      ) : (
        <pre>{preview}</pre>
      )}
      {!details && serialized.length > preview.length ? (
        <span className="file-meta">
          Partial preview · open to inspect the complete value
        </span>
      ) : null}
    </div>
  );
}
