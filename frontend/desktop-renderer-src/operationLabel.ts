import type { Operation } from "../desktop/contract";

type NamedOperation = Pick<Operation, "name" | "operation">;

export function operationLabel(
  operation: NamedOperation,
  catalog: readonly NamedOperation[],
): string {
  const duplicate = catalog.some(
    (other) =>
      other.operation !== operation.operation && other.name === operation.name,
  );
  if (!duplicate) return operation.name;
  if (operation.operation.endsWith(".encode"))
    return `Encode ${operation.name}`;
  if (operation.operation.endsWith(".decode"))
    return `Decode ${operation.name}`;
  return operation.name;
}
