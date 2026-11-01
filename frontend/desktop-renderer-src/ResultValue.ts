import { stringifyJson } from "../desktop/json-codec";
import type { JsonValue, RunDetail } from "../desktop/workspace-contract";

export function formatResultValue(
  value: JsonValue,
  semanticType?: string,
): string {
  if (
    semanticType === "bytes" &&
    value !== null &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    typeof value.base64 === "string"
  )
    return `Base64 ${value.base64}`;
  if (semanticType === "text" && typeof value === "string") return value;
  return stringifyJson(value, null, 2);
}
export function attemptOutputType(
  run: RunDetail,
  nodeId: string,
  port: string,
): string | undefined {
  if (run.plan.targetNodes.includes(nodeId))
    return run.plan.contract.outputs.find((p) => p.name === port)
      ?.semantic_type;
  const digest = run.plan.nodes.find((n) => n.id === nodeId)?.contractDigest;
  const contract =
    digest === run.plan.contractDigest
      ? run.plan.contract
      : digest
        ? run.plan.contracts[digest]
        : undefined;
  return contract?.outputs.find((p) => p.name === port)?.semantic_type;
}
