import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type {
  ExplorationView,
  WorkspaceRequest,
  WorkspaceView,
} from "../../desktop/workspace-contract";
import { useWorkspace } from "../useWorkspace";

afterEach(() => vi.unstubAllGlobals());

it.each(["content", "exploration"])(
  "refreshes %s without reopening the Workspace during an active Run",
  async (action) => {
    let running = false;
    let opened = 0;
    let exploration: ExplorationView = {
      id: "first",
      title: "First",
      revision: 1,
      workspaceId: "workspace",
      stages: [],
      trails: [],
      stage: {
        id: "stage",
        title: "Stage",
        revision: 1,
        introduction: "",
        blocks: [],
        archivedBlocks: [],
      },
    };
    let workspace: WorkspaceView = {
      id: "workspace",
      title: "Workspace",
      generation: 2,
      selectedExplorationId: "first",
      selectedStageId: "stage",
      explorations: [{ id: "first", title: "First", revision: 1 }],
      pipelines: [],
    };
    const command = vi.fn(async (request: WorkspaceRequest) => {
      switch (request.kind) {
        case "list":
          return { lastWorkspaceId: "workspace", workspaces: [] };
        case "open":
          if (running) throw new Error("busy");
          opened++;
          return workspace;
        case "view":
          return workspace;
        case "open_exploration":
          return exploration;
        case "add_text":
          exploration = {
            ...exploration,
            stage: {
              ...exploration.stage,
              blocks: [
                {
                  kind: "asset",
                  id: "block",
                  asset: {
                    id: "asset",
                    explorationId: "first",
                    revision: 1,
                    title: request.title,
                    content: { kind: "text", text: request.text },
                  },
                },
              ],
            },
          };
          return { revision: 2 };
        case "create_exploration":
          exploration = { ...exploration, id: "second", title: request.title };
          workspace = {
            ...workspace,
            explorations: [
              ...workspace.explorations,
              { id: "second", title: request.title, revision: 1 },
            ],
          };
          return { explorationId: "second", stageId: "stage", revision: 1 };
        default:
          throw new Error(`unexpected command: ${request.kind}`);
      }
    });
    vi.stubGlobal("play", { workspace: command });
    const { result, unmount } = renderHook(() => useWorkspace());
    await waitFor(() => expect(result.current.exploration?.id).toBe("first"));
    running = true;
    await act(async () => {
      if (action === "content") await result.current.addText("Clue", "Uryyb");
      else await result.current.createExploration("Second");
    });
    expect(result.current.error).toBe("");
    expect(result.current.busy).toBe(false);
    expect(opened).toBe(1);
    if (action === "content")
      expect(result.current.exploration?.stage.blocks[0]?.id).toBe("block");
    else {
      expect(result.current.exploration?.id).toBe("second");
      expect(result.current.workspace?.explorations).toHaveLength(2);
    }
    unmount();
  },
);
