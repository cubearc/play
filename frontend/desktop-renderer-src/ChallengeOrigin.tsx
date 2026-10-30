import type { ChallengeOrigin as Origin } from "../desktop/workspace-contract";

export function ChallengeOrigin({ origin }: { origin: Origin | undefined }) {
  if (!origin) return null;
  return (
    <details className="challenge-origin">
      <summary>Imported Challenge · revision {origin.revision}</summary>
      <p>
        Editable local copy. Importing does not verify a solution or create
        official participation.
      </p>
      <dl>
        <dt>Package</dt>
        <dd>{origin.packageId}</dd>
        <dt>SHA-256</dt>
        <dd>{origin.sha256}</dd>
        {origin.remote ? (
          <>
            <dt>Claimed service</dt>
            <dd>{origin.remote.service_id}</dd>
            <dt>Claimed Challenge</dt>
            <dd>{origin.remote.challenge_id}</dd>
            <dt>Claimed edition</dt>
            <dd>{origin.remote.edition_id}</dd>
          </>
        ) : null}
      </dl>
    </details>
  );
}
