import type { Operation } from "./contract.ts";

/** Collect one immutable catalog while keeping every IPC page independently bounded. */
export async function loadCatalog(
  request: (offset: number) => Promise<Record<string, unknown>>,
): Promise<Operation[]> {
  const operations: Operation[] = [];
  const identities = new Set<string>();
  let expected: number | undefined;
  for (;;) {
    const page = await request(operations.length);
    if (
      page.type !== "catalog" ||
      page.offset !== operations.length ||
      !Number.isSafeInteger(page.total) ||
      Number(page.total) < 0 ||
      Number(page.total) > 512 ||
      (expected !== undefined && page.total !== expected) ||
      !Array.isArray(page.operations) ||
      page.operations.length > 8 ||
      (!page.operations.length && page.total !== 0)
    )
      throw new Error("invalid_catalog");
    expected = Number(page.total);
    for (const operation of page.operations as Operation[]) {
      if (
        !operation ||
        typeof operation.operation !== "string" ||
        identities.has(operation.operation) ||
        !Number.isSafeInteger(operation.operation_version) ||
        operation.operation_version < 1 ||
        !["available", "unavailable"].includes(operation.availability?.state) ||
        (operation.availability.state === "unavailable" &&
          typeof operation.availability.reason !== "string")
      )
        throw new Error("invalid_catalog");
      identities.add(operation.operation);
      operations.push(operation);
    }
    if (operations.length > expected) throw new Error("invalid_catalog");
    if (page.nextOffset === null) {
      if (operations.length !== expected) throw new Error("invalid_catalog");
      return operations;
    }
    if (page.nextOffset !== operations.length || operations.length >= expected)
      throw new Error("invalid_catalog");
  }
}
