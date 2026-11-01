import { act, fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, expect, it, vi } from "vitest";
import { ExportButton } from "./ExportButton";

vi.mock("./GraphInputs", () => ({ Select: () => null }));
vi.mock("./VscodeButton", () => ({
  default: ({
    children,
    onClick,
    disabled,
  }: {
    children: ReactNode;
    onClick: () => void;
    disabled?: boolean;
  }) => (
    <button type="button" disabled={disabled} onClick={onClick}>
      {children}
    </button>
  ),
}));
afterEach(() => vi.unstubAllGlobals());
it("an export response cannot attach to a different Asset revision", async () => {
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.open = true;
    },
  });
  const context = {
    workspace_id: crypto.randomUUID(),
    exploration_id: crypto.randomUUID(),
    stage_id: crypto.randomUUID(),
  };
  const source = {
    kind: "asset" as const,
    asset_id: crypto.randomUUID(),
    revision: 1,
  };
  let resolve:
    | ((value: { fileName: string; size: number; sha256: string }) => void)
    | undefined;
  const exportSelection = vi.fn(
    () =>
      new Promise<{ fileName: string; size: number; sha256: string }>(
        (done) => {
          resolve = done;
        },
      ),
  );
  vi.stubGlobal("play", { exportSelection });
  const view = render(
    <ExportButton context={context} source={source} label="Clue" />,
  );
  fireEvent.click(screen.getByText("Export Clue"));
  fireEvent.click(screen.getByText("Choose destination"));
  expect(exportSelection).toHaveBeenCalledExactlyOnceWith({
    context,
    source,
    format: "data",
  });
  view.rerender(
    <ExportButton
      context={context}
      source={{ ...source, revision: 2 }}
      label="Clue"
    />,
  );
  await act(async () =>
    resolve?.({ fileName: "old.txt", size: 4, sha256: "a".repeat(64) }),
  );
  expect(screen.queryByText(/Exported old.txt/)).toBeNull();
});
