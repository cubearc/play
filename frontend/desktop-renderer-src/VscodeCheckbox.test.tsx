import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { expect, it, vi } from "vitest";
import VscodeCheckbox from "./VscodeCheckbox";

function Selection({
  disabled = false,
  onChange,
}: {
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}) {
  const [selected, setSelected] = useState(false);
  return (
    <>
      <VscodeCheckbox
        checked={selected}
        disabled={disabled}
        onChange={(event) => {
          setSelected(event.currentTarget.checked);
          onChange(event.currentTarget.checked);
        }}
      >
        Include saved note
      </VscodeCheckbox>
      <output>{selected ? "Included" : "Excluded"}</output>
    </>
  );
}

it("keeps label and native-control activation aligned with the retained selection", async () => {
  const user = userEvent.setup();
  const onChange = vi.fn();
  render(<Selection onChange={onChange} />);
  await user.click(screen.getByText("Include saved note"));
  expect(
    screen.getByRole("checkbox", { name: "Include saved note" }),
  ).toBeChecked();
  expect(screen.getByText("Included")).toBeVisible();
  expect(onChange).toHaveBeenCalledExactlyOnceWith(true);
  await user.click(
    screen.getByRole("checkbox", { name: "Include saved note" }),
  );
  expect(screen.getByText("Excluded")).toBeVisible();
  expect(onChange).toHaveBeenLastCalledWith(false);
  expect(onChange).toHaveBeenCalledTimes(2);
});

it("supports keyboard focus and Space without a second state", async () => {
  const user = userEvent.setup();
  const onChange = vi.fn();
  render(<Selection onChange={onChange} />);
  await user.tab();
  expect(
    screen.getByRole("checkbox", { name: "Include saved note" }),
  ).toHaveFocus();
  await user.keyboard(" ");
  expect(screen.getByText("Included")).toBeVisible();
  expect(onChange).toHaveBeenCalledExactlyOnceWith(true);
});

it("refuses pointer and keyboard changes when disabled", async () => {
  const user = userEvent.setup();
  const onChange = vi.fn();
  render(<Selection disabled onChange={onChange} />);
  await user.click(screen.getByText("Include saved note"));
  await user.tab();
  await user.keyboard(" ");
  expect(
    screen.getByRole("checkbox", { name: "Include saved note" }),
  ).toBeDisabled();
  expect(screen.getByText("Excluded")).toBeVisible();
  expect(onChange).not.toHaveBeenCalled();
});
