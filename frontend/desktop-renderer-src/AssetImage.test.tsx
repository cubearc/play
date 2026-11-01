import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { Context, ImageRead } from "../desktop/workspace-contract";
import { AssetImage, ImagePreview } from "./AssetImage";

const context: Context = {
  workspace_id: crypto.randomUUID(),
  exploration_id: crypto.randomUUID(),
  stage_id: crypto.randomUUID(),
};
it("changed image context cannot display a late preview from the previous selection", async () => {
  const waiting: ((value: string) => void)[] = [];
  const image = vi.fn(
    (_request: ImageRead) =>
      new Promise<string>((resolve) => waiting.push(resolve)),
  );
  vi.stubGlobal("play", { image });
  const source = {
    kind: "run_output" as const,
    run_id: crypto.randomUUID(),
    attempt_id: crypto.randomUUID(),
    port: "output",
  };
  const view = render(
    <ImagePreview context={context} source={source} title="Derived image" />,
  );
  await waitFor(() => expect(image).toHaveBeenCalledTimes(1));
  expect(image).toHaveBeenLastCalledWith({ context, source });
  const other = { ...context, stage_id: crypto.randomUUID() };
  view.rerender(
    <ImagePreview context={other} source={source} title="Derived image" />,
  );
  await waitFor(() => expect(image).toHaveBeenCalledTimes(2));
  await act(async () => waiting[0]?.("cube-play://app/assets/old"));
  expect(screen.queryByRole("img")).toBeNull();
  await act(async () => waiting[1]?.("cube-play://app/assets/current"));
  expect(screen.getByRole("img", { name: "Derived image" })).toHaveAttribute(
    "src",
    "cube-play://app/assets/current",
  );
  fireEvent.error(screen.getByRole("img"));
  expect(screen.getByText(/Image unavailable/)).toBeVisible();
});
it("Asset previews request their exact revision and explain unavailable resources", async () => {
  const image = vi.fn().mockRejectedValue(new Error("not_found"));
  vi.stubGlobal("play", { image });
  const asset = {
    id: crypto.randomUUID(),
    explorationId: context.exploration_id,
    revision: 3,
    title: "Saved figure",
    content: { kind: "image" as const },
  };
  render(<AssetImage asset={asset} context={context} />);
  await screen.findByText(/Image unavailable/);
  expect(image).toHaveBeenCalledExactlyOnceWith({
    context,
    source: { kind: "asset", asset_id: asset.id, revision: 3 },
  });
  expect(screen.queryByRole("img")).toBeNull();
});
