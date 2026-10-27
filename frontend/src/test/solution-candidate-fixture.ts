import { id } from "./lab-fixture";
export const challengeKey = "b".repeat(64);
export function localChallenge() {
  return {
    schema_version: 1,
    challenges: [{ key: challengeKey, id: "local", title: "Local" }],
    active: {
      definition: {
        key: challengeKey,
        store: "source",
        path: "rabbit.cube",
        manifest: {
          id: "local",
          title: "Local",
          description: "",
          stages: ["one", "two"].map((id) => ({
            id,
            title: `Stage ${id}`,
            instructions: "Inspect",
            steps: [],
            hints: [],
            artifacts: [],
            answer_type: "text",
          })),
        },
      },
      stage: "one",
      revision: 1,
      artifacts: [],
      progress: [],
    },
  };
}
export function localCandidate(revision = 1) {
  return {
    revision,
    result_id: id,
    representation: "text",
    byte_size: 2,
    sha256: "a".repeat(64),
    artifact_sha256: "b".repeat(64),
    artifact_operation: "cipher.rot5",
    core_version: "0.18.0",
    selection: { kind: "field", pointer: "/text", conversion: "text" },
    status: "private_unverified",
  };
}
export function localCandidates() {
  return {
    schema_version: 1,
    candidate: localCandidate(2),
    history: [localCandidate(2), localCandidate(1)],
    error: "The artifact needs recovery.",
  };
}
