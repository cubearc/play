import VscodeCollapsible from "@vscode-elements/react-elements/dist/components/VscodeCollapsible.js";
import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import type {
  ExplorationView,
  TrailView,
  WorkspaceView,
} from "../desktop/workspace-contract";
import { CandidateSummary } from "./Candidates";
import VscodeButton from "./VscodeButton";

export function Explorer({
  hidden,
  workspace,
  exploration,
  activeTab,
  busy,
  selectExploration,
  selectStage,
  openNotebook,
  openTrail,
  newExploration,
  importChallenge,
  newStage,
  chooseWorkspace,
}: {
  hidden: boolean;
  workspace: WorkspaceView | null;
  exploration: ExplorationView | null;
  activeTab: string;
  busy: boolean;
  selectExploration: (id: string) => void;
  selectStage: (id: string) => void;
  openNotebook: () => void;
  openTrail: (trail: TrailView) => void;
  newExploration: () => void;
  importChallenge: () => void;
  newStage: () => void;
  chooseWorkspace: () => void;
}) {
  return (
    <aside className="explorer" aria-label="Explorer" hidden={hidden}>
      <h2 className="sidebar-title">Explorer</h2>
      <div className="workspace-heading">
        <VscodeIcon name="root-folder" />
        <strong>{workspace?.title ?? "Cube Play"}</strong>
      </div>
      <div className="workspace-label">
        {workspace ? "Private local workspace" : "No workspace open"}
      </div>
      <div className="explorer-actions">
        <VscodeButton
          secondary
          icon="folder-opened"
          disabled={busy}
          onClick={chooseWorkspace}
        >
          Workspaces
        </VscodeButton>
        {workspace ? (
          <VscodeButton
            secondary
            icon="add"
            disabled={busy}
            onClick={newExploration}
          >
            New Exploration
          </VscodeButton>
        ) : null}
        {workspace ? (
          <VscodeButton
            secondary
            icon="folder-opened"
            disabled={busy}
            onClick={importChallenge}
          >
            Import Challenge
          </VscodeButton>
        ) : null}
      </div>
      {workspace ? (
        <>
          <VscodeCollapsible heading="EXPLORATIONS" open>
            <nav className="exploration-tree" aria-label="Explorations">
              {workspace.explorations.map((entry) => (
                <div key={entry.id}>
                  <button
                    type="button"
                    className={`tree-row exploration-row ${entry.id === exploration?.id ? "current" : ""}`}
                    disabled={busy}
                    aria-current={
                      entry.id === exploration?.id ? "true" : undefined
                    }
                    onClick={() => selectExploration(entry.id)}
                  >
                    <VscodeIcon
                      name={
                        entry.id === exploration?.id
                          ? "chevron-down"
                          : "chevron-right"
                      }
                    />
                    <VscodeIcon name="beaker" />
                    {entry.title}
                  </button>
                  {entry.id === exploration?.id ? (
                    <div className="exploration-children">
                      {exploration.stages.map((stage, index) => (
                        <button
                          key={stage.id}
                          type="button"
                          className={`tree-row ${activeTab === "notebook" && stage.id === exploration.stage.id ? "selected" : ""}`}
                          disabled={busy}
                          onClick={() => {
                            if (stage.id === exploration.stage.id)
                              openNotebook();
                            else selectStage(stage.id);
                          }}
                        >
                          <VscodeIcon name="notebook" />
                          <span>
                            {String(index + 1).padStart(2, "0")} · {stage.title}
                          </span>
                        </button>
                      ))}
                      <button
                        type="button"
                        className="tree-row"
                        disabled={busy}
                        onClick={newStage}
                      >
                        <VscodeIcon name="add" />
                        <span>New Stage</span>
                      </button>
                      {exploration.trails.some((trail) => !trail.archived) ? (
                        <div className="tree-section">Trails</div>
                      ) : null}
                      {exploration.trails
                        .filter((trail) => !trail.archived)
                        .map((trail) => (
                          <button
                            key={trail.id}
                            type="button"
                            className={`tree-row trail-row ${activeTab === trail.id ? "selected" : ""}`}
                            onClick={() => openTrail(trail)}
                          >
                            <VscodeIcon name="git-branch" />
                            <span>{trail.title}</span>
                          </button>
                        ))}
                      {exploration.trails.some((trail) => trail.archived) ? (
                        <div className="tree-section">Archived trails</div>
                      ) : null}
                      {exploration.trails
                        .filter((trail) => trail.archived)
                        .map((trail) => (
                          <button
                            key={trail.id}
                            type="button"
                            className={`tree-row trail-row ${activeTab === trail.id ? "selected" : ""}`}
                            aria-label={`Archived ${trail.title}`}
                            onClick={() => openTrail(trail)}
                          >
                            <VscodeIcon name="archive" />
                            <span>{trail.title}</span>
                          </button>
                        ))}
                    </div>
                  ) : null}
                </div>
              ))}
              {!workspace.explorations.length ? (
                <p className="sidebar-note muted">
                  Create an Exploration to start your notebook.
                </p>
              ) : null}
            </nav>
          </VscodeCollapsible>
          <VscodeCollapsible heading="WORKSPACE PIPELINES" open>
            <div className="pipeline-reference">
              {workspace.pipelines.map((pipeline) => (
                <div key={pipeline.id}>
                  <span className="tree-label">
                    <VscodeIcon name="type-hierarchy-sub" />
                    {pipeline.title}
                    <span className="badge">r{pipeline.revision}</span>
                  </span>
                </div>
              ))}
              <p>
                {workspace.pipelines.length
                  ? "Reusable definitions. Each trail binds its own source asset."
                  : "Text blocks create a reusable text pipeline when added."}
              </p>
            </div>
          </VscodeCollapsible>
          <VscodeCollapsible
            className="evidence-section"
            heading="EVIDENCE"
            open
          >
            <div className="evidence-empty">
              <VscodeIcon name="bookmark" />
              <CandidateSummary />
            </div>
            <p className="sidebar-note muted">
              Content, saved reasoning and retained Run results persist locally.
            </p>
          </VscodeCollapsible>
        </>
      ) : null}
    </aside>
  );
}
