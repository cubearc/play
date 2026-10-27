import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { resolved, SchemaField } from "./SchemaField";

it("offers accessible choices and explicit private controls", async () => {
  const user = userEvent.setup(),
    changed = vi.fn();
  render(
    <SchemaField
      label="Direction"
      schema={{ type: "string", enum: ["left", "right"] }}
      value="left"
      onChange={changed}
    />,
  );
  await user.selectOptions(
    screen.getByRole("combobox", { name: "Direction" }),
    "1",
  );
  expect(changed).toHaveBeenLastCalledWith("right");
});
it("keeps a large integer as an exact decimal string", async () => {
  const user = userEvent.setup(),
    changed = vi.fn();
  render(
    <SchemaField
      label="Operand"
      schema={{ "x-cube-integer": { type: "integer" } }}
      value={undefined}
      onChange={changed}
    />,
  );
  await user.paste("18446744073709551615");
  await user.click(screen.getByLabelText("Operand"));
  await user.paste("18446744073709551615");
  expect(changed).toHaveBeenLastCalledWith("18446744073709551615");
});
it("resolves local typed definitions and rejects cyclic aliases", () => {
  expect(
    resolved({ $ref: "#/$defs/Text" }, { $defs: { Text: { type: "string" } } }),
  ).toEqual({ type: "string" });
  expect(() =>
    resolved(
      { $ref: "#/$defs/Loop" },
      { $defs: { Loop: { $ref: "#/$defs/Loop" } } },
    ),
  ).toThrow();
});

it("distinguishes an explicit empty string from an unset field without retaining old text", async () => {
  const user = userEvent.setup(),
    changed = vi.fn();
  const { rerender } = render(
    <SchemaField
      label="keyword"
      schema={{ type: "string" }}
      value={undefined}
      onChange={changed}
    />,
  );
  await user.click(
    screen.getByRole("button", { name: "Use empty text for keyword" }),
  );
  expect(changed).toHaveBeenLastCalledWith("");
  rerender(
    <SchemaField
      label="keyword"
      schema={{ type: "string" }}
      value="A"
      onChange={changed}
    />,
  );
  await user.clear(screen.getByLabelText("keyword"));
  expect(changed).toHaveBeenLastCalledWith("");
  rerender(
    <SchemaField
      label="source"
      schema={{ type: "string", minLength: 1 }}
      value={undefined}
      onChange={changed}
    />,
  );
  expect(
    screen.queryByRole("button", { name: /Use empty text/ }),
  ).not.toBeInTheDocument();
});
