import { afterEach, expect, it, vi } from "vitest";
import type { WorkspaceView } from "../desktop/workspace-contract";
import { locationFor, saveLocation } from "./navigation";

const stage = "33333333-3333-4333-8333-333333333333";
const workspace: WorkspaceView = {
  id: "workspace",
  title: "Local",
  generation: 2,
  pipelines: [],
  explorations: [
    { id: "a", title: "A", revision: 1 },
    { id: "b", title: "B", revision: 1 },
  ],
  selectedExplorationId: "b",
  selectedStageId: null,
};
afterEach(() => {
  sessionStorage.clear();
  vi.restoreAllMocks();
});
it("restores this window's location instead of another window's most recent selection", () => {
  saveLocation(workspace.id, "a", stage);
  expect(locationFor(workspace)).toEqual({
    explorationId: "a",
    stageId: stage,
  });
  expect(locationFor({ ...workspace, id: "another workspace" })).toBeNull();
});
it.each([
  "{",
  "x".repeat(257),
  JSON.stringify({ explorationId: "foreign", stageId: stage }),
  JSON.stringify({ explorationId: "a", stageId: "invalid" }),
  JSON.stringify({ explorationId: "a", stageId: stage, run: true }),
])("ignores an invalid or foreign stored location", (value) => {
  sessionStorage.setItem("play.location.workspace", value);
  expect(locationFor(workspace)).toBeNull();
});
it("storage refusal does not prevent current navigation", () => {
  vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("denied");
  });
  expect(() => saveLocation(workspace.id, "a", stage)).not.toThrow();
  vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
    throw new Error("denied");
  });
  expect(locationFor(workspace)).toBeNull();
});
