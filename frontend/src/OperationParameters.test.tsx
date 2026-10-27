import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { expect, it } from "vitest";
import { type Contract, type Json, parseCatalog } from "./exploration-api";
import { OperationParameters } from "./OperationParameters";

it.each(["cipher.substitution.encode", "cipher.substitution.decode"])(
  "edits both real canonical substitution forms for %s",
  async (operation) => {
    const document = JSON.parse(
      readFileSync(
        resolve(process.cwd(), "../../cube/contracts/v1/catalog.json"),
        "utf8",
      ),
    );
    const contract = parseCatalog({
      schema_version: 2,
      contract_schema_version: 1,
      operations: document.operations
        .filter((c: { operation: string }) => c.operation === operation)
        .map((contract: unknown) => ({
          contract,
          execution: { available: true, reason: null, adapter_version: 2 },
        })),
    })[0]?.contract;
    if (!contract) throw new Error("Core substitution contract missing");
    const user = userEvent.setup();
    render(<Example contract={contract} />);
    await user.type(screen.getByLabelText(/^source/), "ABC");
    await user.type(screen.getByLabelText(/^target/), "CAB");
    expect(
      JSON.parse(
        screen.getByLabelText("Configured parameters").textContent ?? "{}",
      ),
    ).toEqual({ source: "ABC", target: "CAB" });
    await user.selectOptions(screen.getByLabelText("Parameter form"), "1");
    expect(screen.queryByLabelText(/^target/)).not.toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Use empty text for keyword" }),
    );
    expect(
      JSON.parse(
        screen.getByLabelText("Configured parameters").textContent ?? "{}",
      ).keyword,
    ).toBe("");
    await user.type(screen.getByLabelText(/^keyword/), "CAB");
    expect(
      JSON.parse(
        screen.getByLabelText("Configured parameters").textContent ?? "{}",
      ),
    ).toEqual({ source: "ABC", keyword: "CAB", unmapped: "reject" });
    await user.clear(screen.getByLabelText(/^keyword/));
    expect(
      JSON.parse(
        screen.getByLabelText("Configured parameters").textContent ?? "{}",
      ).keyword,
    ).toBe("");
  },
);

function Example({ contract }: { contract: Contract }) {
  const [values, setValues] = useState<Record<string, Json>>({});
  return (
    <>
      <OperationParameters
        contract={contract}
        values={values}
        privateValues={{}}
        onChange={setValues}
        onPrivateChange={() => {}}
        onValidityChange={() => {}}
      />
      <output aria-label="Configured parameters">
        {JSON.stringify(values)}
      </output>
    </>
  );
}
