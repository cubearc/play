import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, expect, it } from "vitest";
import type { Panel } from "./investigation-api";
import { WorkspacePanels } from "./WorkspacePanels";

const width = Object.getOwnPropertyDescriptor(window, "innerWidth");
afterEach(() => {
  if (width) Object.defineProperty(window, "innerWidth", width);
});
function Harness() {
  const [panel, setPanel] = useState<Panel>("files");
  return (
    <WorkspacePanels active={panel} onChange={setPanel}>
      {{
        files: (
          <>
            <button type="button" onClick={() => setPanel("solutions")}>
              Preserve this finding
            </button>
            <label>
              File draft
              <input defaultValue="" />
            </label>
          </>
        ),
        explorations: (
          <label>
            Exploration draft
            <input defaultValue="" />
          </label>
        ),
        notes: <p>Private observations</p>,
        artifacts: <p>Preserved artifacts</p>,
        solutions: <p>Private findings</p>,
        challenges: <p>Local instructions</p>,
        terminal: <p>Real Cube terminal</p>,
      }}
    </WorkspacePanels>
  );
}
it("switches accessible sections without unmounting drafts and supports vertical keyboard navigation", async () => {
  Object.defineProperty(window, "innerWidth", {
    value: 1200,
    configurable: true,
  });
  const user = userEvent.setup();
  render(<Harness />);
  await user.type(
    screen.getByRole("textbox", { name: "File draft" }),
    "private buffer",
  );
  await user.click(screen.getByRole("tab", { name: "Explorations" }));
  expect(screen.getByRole("tabpanel", { name: "Explorations" })).toBeVisible();
  expect(screen.queryByRole("textbox", { name: "File draft" })).toBeNull();
  await user.keyboard("{ArrowDown}");
  expect(screen.getByRole("tab", { name: "Notes" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await user.keyboard("{Home}");
  expect(screen.getByRole("textbox", { name: "File draft" })).toHaveValue(
    "private buffer",
  );
  await user.keyboard("{End}");
  expect(screen.getByRole("tabpanel", { name: "Terminal" })).toBeVisible();
});
it("moves keyboard focus to the destination when a panel action opens another section", async () => {
  const user = userEvent.setup();
  render(<Harness />);
  await user.click(
    screen.getByRole("button", { name: "Preserve this finding" }),
  );
  expect(screen.getByRole("tab", { name: "Solutions" })).toHaveFocus();
  expect(screen.getByRole("tabpanel", { name: "Solutions" })).toBeVisible();
});
it("uses horizontal arrow navigation on narrow layouts and updates orientation on resize", async () => {
  Object.defineProperty(window, "innerWidth", {
    value: 390,
    configurable: true,
    writable: true,
  });
  const user = userEvent.setup();
  render(<Harness />);
  const tabs = screen.getByRole("tablist", { name: "Workspace sections" });
  expect(tabs).toHaveAttribute("aria-orientation", "horizontal");
  await user.click(screen.getByRole("tab", { name: "Files" }));
  await user.keyboard("{ArrowLeft}");
  expect(screen.getByRole("tab", { name: "Terminal" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await act(async () => {
    Object.defineProperty(window, "innerWidth", {
      value: 1200,
      configurable: true,
    });
    window.dispatchEvent(new Event("resize"));
  });
  expect(tabs).toHaveAttribute("aria-orientation", "vertical");
});
