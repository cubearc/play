import VscodeCollapsible from "@vscode-elements/react-elements/dist/components/VscodeCollapsible.js";
import VscodeIcon from "@vscode-elements/react-elements/dist/components/VscodeIcon.js";
import {
  type ExplorationView,
  exampleWorkspace,
  type TrailView,
  textPipeline,
} from "./exampleWorkspace";

export function Explorer({
  hidden,
  exploration,
  activeTab,
  selectExploration,
  openNotebook,
  openTrail,
}: {
  hidden: boolean;
  exploration: ExplorationView;
  activeTab: string;
  selectExploration: (entry: ExplorationView) => void;
  openNotebook: () => void;
  openTrail: (trail: TrailView) => void;
}) {
  return (
    <aside className="explorer" aria-label="Explorer" hidden={hidden}>
      <h2 className="sidebar-title">Explorer</h2>
      <div className="workspace-heading">
        <VscodeIcon name="root-folder" />
        <strong>{exampleWorkspace.title}</strong>
      </div>
      <div className="workspace-label">Example workspace · session only</div>
      <VscodeCollapsible heading="EXPLORATIONS" open>
        <nav className="exploration-tree" aria-label="Explorations">
          {exampleWorkspace.explorations.map((entry) => (
            <div key={entry.id}>
              <button
                type="button"
                className={`tree-row exploration-row ${entry.id === exploration.id ? "current" : ""}`}
                aria-current={entry.id === exploration.id ? "true" : undefined}
                onClick={() => selectExploration(entry)}
              >
                <VscodeIcon
                  name={
                    entry.id === exploration.id
                      ? "chevron-down"
                      : "chevron-right"
                  }
                />
                <VscodeIcon name="beaker" />
                {entry.title}
              </button>
              {entry.id === exploration.id ? (
                <div className="exploration-children">
                  <button
                    type="button"
                    className={`tree-row ${activeTab === "notebook" ? "selected" : ""}`}
                    onClick={openNotebook}
                  >
                    <VscodeIcon name="notebook" />
                    <span>01 · {entry.stage.title}</span>
                  </button>
                  <div className="tree-section">Trails</div>
                  {entry.trails.map((trail) => (
                    <button
                      type="button"
                      key={trail.id}
                      className={`tree-row trail-row ${activeTab === trail.id ? "selected" : ""}`}
                      onClick={() => openTrail(trail)}
                    >
                      <VscodeIcon name="git-branch" />
                      <span>{trail.title}</span>
                    </button>
                  ))}
                </div>
              ) : null}
            </div>
          ))}
        </nav>
      </VscodeCollapsible>
      <VscodeCollapsible heading="WORKSPACE PIPELINES" open>
        <div className="pipeline-reference">
          <span className="tree-label">
            <VscodeIcon name="type-hierarchy-sub" />
            {textPipeline.title}
            <span className="badge">r{textPipeline.revision}</span>
          </span>
          <p>Reusable definition. Each trail binds its own source asset.</p>
        </div>
      </VscodeCollapsible>
      <VscodeCollapsible className="evidence-section" heading="EVIDENCE" open>
        <div className="evidence-empty">
          <VscodeIcon name="bookmark" />
          <span>No candidates or solutions saved.</span>
        </div>
        <p className="sidebar-note muted">
          Results and reasoning in this preview remain in this session.
        </p>
      </VscodeCollapsible>
    </aside>
  );
}
