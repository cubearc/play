import { useCallback, useEffect, useRef, useState } from "react";
import type {
  BlockView,
  Context,
  ExplorationView,
  NoteView,
  WorkspaceList,
  WorkspaceRequest,
  WorkspaceView,
} from "../desktop/workspace-contract";
import {
  blockSource,
  contextOf,
  reasoningOf,
} from "../desktop/workspace-contract";

const messages: Record<string, string> = {
  result_read_only:
    "Saved results preserve their original output. Duplicate this block to edit an alternative.",
  result_not_available:
    "Only a completed Run with a retained output can be saved as a block.",
  result_conflict:
    "Notebook content changed. The latest state is shown and your reasoning draft was kept. Review it, then save the result again.",
  lifecycle_conflict:
    "Notebook or saved block changed. The latest state is shown and your reasoning draft was kept. Review it, then try again.",
  block_archived:
    "This block is archived. Restore it from Archived blocks to continue editing, copying or running it.",
  retained_block_limit:
    "This Stage reached its retained-block limit. Restore existing content or create another Stage.",
  copy_conflict:
    "Saved block changed or the notebook was reorganized. The latest content is shown and your reasoning draft was kept. Review it, then duplicate again.",
  order_conflict:
    "Notebook order changed. The latest order is shown and your draft was kept. Try the move again.",
  unsupported_store:
    "This workspace uses an unsupported or damaged storage format. Existing data was preserved. Create a separate workspace or reopen it with its supported version.",
  workspace_busy:
    "This workspace is already open in another engine. Close that owner before reopening it here.",
  state_conflict:
    "The saved content changed. Your draft was kept; cancel and reopen the editor to review the latest revision.",
  storage_quota:
    "A notebook or workspace storage limit was reached. Existing content was preserved.",
  input_limit: "This content exceeds the supported text limit.",
  image_limit: "Choose a PNG image no larger than 128 KiB.",
  file_limit: "Choose a file no larger than 128 KiB.",
  invalid_file: "Choose a regular file that can be read.",
  invalid_title:
    "Use a non-empty title or filename of at most 120 UTF-8 bytes.",
  invalid_image: "This file is not a supported bounded PNG image.",
  workspace_mismatch: "This request belongs to another workspace.",
  unsafe_store_permissions:
    "The private store has unsafe permissions. Existing data was preserved.",
  store_missing: "Workspace storage is missing. It was not recreated or reset.",
};
export function errorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return (
    Object.entries(messages).find(([code]) => message.includes(code))?.[1] ??
    "The workspace request could not be completed. Existing data was preserved."
  );
}
interface Draft {
  context: Context;
  noteId: string;
  revision: number;
  value: string;
  key: string;
}
export function useWorkspace() {
  const [list, setList] = useState<WorkspaceList>({
    workspaces: [],
    lastWorkspaceId: null,
  });
  const [workspace, setWorkspace] = useState<WorkspaceView | null>(null);
  const [exploration, setExploration] = useState<ExplorationView | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState("");
  const [drafts, setDrafts] = useState<Record<string, Draft>>({});
  const [saving, setSaving] = useState<Record<string, boolean>>({});
  const actionBusy = useRef(false);
  const copyIntents = useRef(
    new Map<string, Extract<WorkspaceRequest, { kind: "duplicate_block" }>>(),
  );
  const resultIntents = useRef(
    new Map<string, Extract<WorkspaceRequest, { kind: "pin_run_output" }>>(),
  );
  const epoch = useRef(0);
  const live = useRef(true);
  const workspaceRef = useRef(workspace);
  workspaceRef.current = workspace;
  const explorationRef = useRef(exploration);
  explorationRef.current = exploration;
  const execute = useCallback(
    async (action: () => Promise<void> | Promise<boolean | undefined>) => {
      if (actionBusy.current) return false;
      actionBusy.current = true;
      setBusy(true);
      setError("");
      try {
        return (await action()) !== false;
      } catch (error) {
        if (live.current) setError(errorMessage(error));
        return false;
      } finally {
        actionBusy.current = false;
        if (live.current) setBusy(false);
      }
    },
    [],
  );
  const load = useCallback(
    async (
      workspaceId: string,
      explorationId: string,
      stageId: string | null,
    ) => {
      const current = ++epoch.current;
      const data = await window.play.workspace({
        kind: "open_exploration",
        workspace_id: workspaceId,
        exploration_id: explorationId,
        stage_id: stageId,
      });
      if (
        live.current &&
        epoch.current === current &&
        workspaceRef.current?.id === workspaceId
      ) {
        setExploration(data);
        setDrafts((previous) => {
          const notes = [
            ...data.stage.blocks,
            ...data.stage.archivedBlocks.map((entry) => entry.block),
          ].filter((block) => block.kind === "note");
          const next = { ...previous };
          for (const block of notes)
            if (next[block.note.id]?.value === block.note.text)
              delete next[block.note.id];
          return next;
        });
        return data;
      }
      return null;
    },
    [],
  );
  const bind = useCallback(
    async (data: WorkspaceView) => {
      if (!live.current) return;
      epoch.current++;
      workspaceRef.current = data;
      setWorkspace(data);
      setExploration(null);
      if (data.selectedExplorationId)
        await load(data.id, data.selectedExplorationId, data.selectedStageId);
    },
    [load],
  );
  const open = useCallback(
    (id: string) =>
      execute(async () =>
        bind(await window.play.workspace({ kind: "open", workspace_id: id })),
      ),
    [bind, execute],
  );
  useEffect(() => {
    live.current = true;
    void execute(async () => {
      const data = await window.play.workspace({ kind: "list" });
      if (!live.current) return;
      setList(data);
      if (data.lastWorkspaceId)
        await bind(
          await window.play.workspace({
            kind: "open",
            workspace_id: data.lastWorkspaceId,
          }),
        );
    });
    return () => {
      live.current = false;
      epoch.current++;
    };
  }, [bind, execute]);
  const refreshSummary = useCallback(async (workspaceId: string) => {
    if (workspaceRef.current?.id !== workspaceId) return;
    try {
      const data = await window.play.workspace({
        kind: "view",
        workspace_id: workspaceId,
      });
      if (live.current && workspaceRef.current?.id === workspaceId) {
        workspaceRef.current = data;
        setWorkspace(data);
      }
    } catch {
      /* A newer workspace selection supplies its own summary. */
    }
  }, []);
  const createWorkspace = (title: string, key: string = crypto.randomUUID()) =>
    execute(async () => {
      const data = await window.play.workspace({ kind: "create", key, title });
      await bind(data);
      setList(await window.play.workspace({ kind: "list" }));
    });
  const createExploration = (
    title: string,
    key: string = crypto.randomUUID(),
  ) =>
    execute(async () => {
      const current = workspaceRef.current;
      if (!current) return;
      const created = await window.play.workspace({
        kind: "create_exploration",
        workspace_id: current.id,
        key,
        title,
      });
      const data = await window.play.workspace({
        kind: "open",
        workspace_id: current.id,
      });
      workspaceRef.current = data;
      setWorkspace(data);
      await load(current.id, created.explorationId, created.stageId);
    });
  const createStage = (title: string, key: string = crypto.randomUUID()) =>
    execute(async () => {
      if (!exploration) return;
      const context = contextOf(exploration);
      const created = await window.play.workspace({
        kind: "create_stage",
        workspace_id: context.workspace_id,
        exploration_id: context.exploration_id,
        key,
        title,
      });
      await load(context.workspace_id, context.exploration_id, created.stageId);
    });
  const selectExploration = (id: string) =>
    execute(async () => {
      const current = workspaceRef.current;
      if (current) await load(current.id, id, null);
    });
  const selectStage = (id: string) =>
    execute(async () => {
      if (exploration) await load(exploration.workspaceId, exploration.id, id);
    });
  const refresh = async (context: Context) => {
    if (workspaceRef.current?.id !== context.workspace_id) return;
    const data = await window.play.workspace({
      kind: "open",
      workspace_id: context.workspace_id,
    });
    workspaceRef.current = data;
    setWorkspace(data);
    await load(context.workspace_id, context.exploration_id, context.stage_id);
  };
  const addText = (
    title: string,
    text: string,
    key: string = crypto.randomUUID(),
  ) =>
    execute(async () => {
      if (!exploration) return;
      const context = contextOf(exploration);
      await window.play.workspace({
        kind: "add_text",
        context,
        key,
        title,
        text,
      });
      await refresh(context);
    });
  const importImage = (caption: string, key: string = crypto.randomUUID()) =>
    execute(async () => {
      if (!exploration) return;
      const context = contextOf(exploration);
      const result = await window.play.importImage({ context, key, caption });
      if (!result) return false;
      await refresh(context);
    });
  const reloadCurrent = async (context: Context) => {
    const shown = explorationRef.current;
    if (
      shown?.workspaceId === context.workspace_id &&
      shown.id === context.exploration_id &&
      shown.stage.id === context.stage_id
    )
      return await load(
        context.workspace_id,
        context.exploration_id,
        context.stage_id,
      );
  };
  const importFile = (key: string = crypto.randomUUID()) =>
    execute(async () => {
      if (!exploration) return;
      const context = contextOf(exploration);
      const result = await window.play.importFile({ context, key });
      if (!result) return false;
      await refresh(context);
    });
  const pinRunOutput = async (
    context: Context,
    revision: number,
    run: string,
    attempt: string,
    port: string,
  ): Promise<string | null> => {
    const slot = `${context.workspace_id}/${context.exploration_id}/${context.stage_id}/${run}/${attempt}/${port}`;
    let block: string | null = null;
    const intent: Extract<WorkspaceRequest, { kind: "pin_run_output" }> =
      resultIntents.current.get(slot) ?? {
        kind: "pin_run_output",
        context,
        key: crypto.randomUUID(),
        expected_revision: revision,
        run_id: run,
        attempt_id: attempt,
        port,
      };
    const accepted = await execute(async () => {
      resultIntents.current.set(slot, intent);
      try {
        const result = await window.play.workspace(intent);
        const latest = await reloadCurrent(context);
        if (!latest) return false;
        resultIntents.current.delete(slot);
        block = result.blockId;
      } catch (error) {
        if (
          error instanceof Error &&
          [
            "state_conflict",
            "result_not_available",
            "not_found",
            "storage_quota",
            "retained_block_limit",
            "workspace_mismatch",
            "contract_mismatch",
            "output_limit",
          ].some((code) => error.message.includes(code))
        )
          resultIntents.current.delete(slot);
        if (
          error instanceof Error &&
          error.message.includes("state_conflict")
        ) {
          await reloadCurrent(context);
          throw new Error("result_conflict");
        }
        throw error;
      }
    });
    return accepted ? block : null;
  };
  const reorderBlocks = (
    context: Context,
    revision: number,
    blockIds: string[],
  ) =>
    execute(async () => {
      try {
        await window.play.workspace({
          kind: "reorder_blocks",
          context,
          key: crypto.randomUUID(),
          expected_revision: revision,
          block_ids: blockIds,
        });
      } catch (error) {
        if (
          error instanceof Error &&
          error.message.includes("state_conflict")
        ) {
          await reloadCurrent(context);
          throw new Error("order_conflict");
        }
        throw error;
      }
      await reloadCurrent(context);
    });
  const setArchived = (
    context: Context,
    revision: number,
    block: BlockView,
    archived: boolean,
  ) =>
    execute(async () => {
      try {
        await window.play.workspace({
          kind: archived ? "archive_block" : "restore_block",
          context,
          key: crypto.randomUUID(),
          block_id: block.id,
          expected_revision: revision,
          source: blockSource(block),
        });
      } catch (error) {
        if (
          error instanceof Error &&
          error.message.includes("state_conflict")
        ) {
          await reloadCurrent(context);
          throw new Error("lifecycle_conflict");
        }
        throw error;
      }
      const latest = await reloadCurrent(context);
      if (!latest) return false;
      if (
        latest.stage.archivedBlocks.some(
          (entry) => entry.block.id === block.id,
        ) !== archived
      )
        throw new Error("lifecycle_conflict");
    });
  const duplicateBlock = async (
    context: Context,
    revision: number,
    block: BlockView,
  ): Promise<string | null> => {
    const slot = `${context.workspace_id}/${context.exploration_id}/${context.stage_id}/${block.id}`;
    const intent: Extract<WorkspaceRequest, { kind: "duplicate_block" }> =
      copyIntents.current.get(slot) ?? {
        kind: "duplicate_block",
        context,
        key: crypto.randomUUID(),
        block_id: block.id,
        expected_revision: revision,
        source: blockSource(block),
      };
    let copied: string | null = null;
    const accepted = await execute(async () => {
      copyIntents.current.set(slot, intent);
      try {
        const result = await window.play.workspace(intent);
        await reloadCurrent(context);
        copyIntents.current.delete(slot);
        copied = result.blockId;
      } catch (error) {
        if (
          error instanceof Error &&
          [
            "input_limit",
            "storage_quota",
            "not_found",
            "invalid_block_type",
            "invalid_image",
            "corrupt_store",
            "workspace_mismatch",
            "block_archived",
            "retained_block_limit",
          ].some((code) => error.message.includes(code))
        )
          copyIntents.current.delete(slot);
        if (
          error instanceof Error &&
          error.message.includes("state_conflict")
        ) {
          copyIntents.current.delete(slot);
          await reloadCurrent(context);
          throw new Error("copy_conflict");
        }
        throw error;
      }
    });
    return accepted ? copied : null;
  };
  const reviseText = (
    context: Context,
    blockId: string,
    assetId: string,
    revision: number,
    text: string,
    key: string,
  ) =>
    execute(async () => {
      try {
        await window.play.workspace({
          kind: "revise_text",
          context,
          key,
          block_id: blockId,
          asset_id: assetId,
          expected_revision: revision,
          text,
        });
      } catch (error) {
        if (
          error instanceof Error &&
          (error.message.includes("state_conflict") ||
            error.message.includes("block_archived"))
        )
          await reloadCurrent(context).catch(() => {});
        throw error;
      }
      await reloadCurrent(context);
    });
  const editNote = (note: NoteView, value: string) => {
    if (
      !exploration?.stage.blocks.some(
        (block) => block.kind === "note" && block.note.id === note.id,
      )
    )
      return;
    setDrafts((previous) => {
      const next = { ...previous };
      if (value === note.text) delete next[note.id];
      else
        next[note.id] = {
          ...(next[note.id] ?? {
            context: contextOf(exploration),
            noteId: note.id,
            revision: note.revision,
          }),
          value,
          key: crypto.randomUUID(),
        };
      return next;
    });
  };
  const discardNote = (id: string) => {
    setDrafts((previous) => {
      const next = { ...previous };
      delete next[id];
      return next;
    });
    setError("");
  };
  const saveNote = async (id?: string): Promise<boolean> => {
    const noteId =
      id ?? (exploration ? reasoningOf(exploration.stage).id : null);
    if (!noteId) return false;
    const draft = drafts[noteId];
    if (!draft || saving[noteId]) return false;
    setSaving((previous) => ({ ...previous, [noteId]: true }));
    setError("");
    try {
      const result = await window.play.workspace({
        kind: "save_note",
        context: draft.context,
        key: draft.key,
        note_id: noteId,
        expected_revision: draft.revision,
        text: draft.value,
      });
      setDrafts((previous) => {
        const next = { ...previous };
        if (next[noteId]?.value === draft.value) delete next[noteId];
        return next;
      });
      setExploration((previous) =>
        previous &&
        previous.workspaceId === draft.context.workspace_id &&
        previous.id === draft.context.exploration_id &&
        previous.stage.id === draft.context.stage_id
          ? {
              ...previous,
              stage: {
                ...previous.stage,
                blocks: previous.stage.blocks.map((block) =>
                  block.kind === "note" && block.note.id === noteId
                    ? {
                        ...block,
                        note: {
                          ...block.note,
                          revision: result.revision,
                          text: draft.value,
                        },
                      }
                    : block,
                ),
              },
            }
          : previous,
      );
      return true;
    } catch (error) {
      if (error instanceof Error && error.message.includes("state_conflict")) {
        await reloadCurrent(draft.context).catch(() => {});
        setError(
          "The saved note changed. Your draft was kept. Discard the draft to review the latest saved version.",
        );
      } else setError(errorMessage(error));
      return false;
    } finally {
      setSaving((previous) => ({ ...previous, [noteId]: false }));
    }
  };
  const addAnnotation = (
    title: string,
    text: string,
    key: string = crypto.randomUUID(),
  ) =>
    execute(async () => {
      if (!exploration) return;
      const context = contextOf(exploration);
      await window.play.workspace({
        kind: "add_annotation",
        context,
        key,
        title,
        text,
      });
      await refresh(context);
    });
  return {
    list,
    workspace,
    exploration,
    busy,
    error,
    clearError: () => setError(""),
    createWorkspace,
    refreshSummary,
    open,
    createExploration,
    createStage,
    selectExploration,
    selectStage,
    addText,
    importImage,
    importFile,
    reviseText,
    reorderBlocks,
    duplicateBlock,
    setArchived,
    pinRunOutput,
    hasUnsavedNotes: Object.keys(drafts).length > 0,
    editNote,
    saveNote,
    discardNote,
    addAnnotation,
    noteState: (note: NoteView) => ({
      value: drafts[note.id]?.value ?? note.text,
      dirty: Boolean(drafts[note.id]),
      saving: Boolean(saving[note.id]),
    }),
    noteDirty: Object.keys(drafts).length > 0,
    noteSaving: Object.values(saving).some(Boolean),
  };
}
export type WorkspaceState = ReturnType<typeof useWorkspace>;
