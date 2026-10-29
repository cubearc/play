# Organize Explorations and Trails

<a id="boundaries-and-recovery"></a>
<a id="check-replay-inputs"></a>
<a id="classical-analysis"></a>
<a id="connect-an-image-workflow"></a>
<a id="history-and-annotations"></a>
<a id="inspect-named-results"></a>
<a id="local-api"></a>
<a id="private-inputs-and-retention"></a>
<a id="reuse-named-outputs-and-structured-fields"></a>
<a id="supply-parameters-and-text"></a>
<a id="typed-explorations"></a>

An Exploration is an independent investigation inside a Workspace. It owns its
Stages, Blocks, Assets, reasoning and Trail history. Use **New Exploration** to
start another investigation, and select its Stages in the Explorer. Its content
and selected Stage remain separate when you switch or reopen the Workspace.

## Open an Asset canvas

Click a text Asset, image, caption, file or Page in a Stage notebook. Play opens a **Trail
canvas** tab for that Asset in the same Exploration. A caption's canvas shows its
exact bound Asset revision, selected reusable Pipeline and accepted Run history.
The image's canvas supports inspection, zoom and a [typed first-pixel Run](visual-lab.md#read-the-first-pixel). Page canvases show inert source text. A file's canvas shows an inert
text/hex preview, byte count and SHA-256 digest. Results stay with the cell or
canvas that produced them.


![Caption Trail canvas with its exact source Asset, reusable Pipeline and completed ROT13 Run.](assets/desktop/caption-canvas.png)

Opening the caption creates its own canvas tab inside the same Exploration. [Open full size](assets/desktop/caption-canvas.png).

## Save a hypothesis

Choose **Edit Trail** in a text, caption or image canvas. Write a hypothesis and
choose **Save Trail**. The hypothesis belongs to that Trail and survives restart;
another Exploration keeps its own reasoning and progress.

![Trail editor with a private hypothesis and a saved Pipeline pinned to revision 1.](assets/desktop/trail-editor.png)

Choose a saved revision and keep this Trail's hypothesis. [Open full size](assets/desktop/trail-editor.png).

## Save and reuse a Pipeline

Choose **New Pipeline** to name a reusable single-node definition. Select an
operation by its Core name, review its parameters and choose **Create Pipeline**.
The Trail selects the saved revision. Choose **Run cell** to execute it against
this Trail's bound Asset.

**Edit Pipeline** opens the current head for editing. **Save revision** freezes a
new configuration and selects it only for the current Trail. An identical
configuration keeps the same revision. Other Trails stay pinned to their chosen
revisions; previous Runs keep their evidence and results.

![Trail canvas after selecting Pipeline revision 2, with the preceding ROT13 Run preserved.](assets/desktop/pipeline-revision.png)

The working revision and historical Run keep separate configurations. [Open full size](assets/desktop/pipeline-revision.png).

To reuse a definition in another Exploration, open its Asset canvas, choose
**Edit Trail**, then choose **Saved Pipeline** and **Pipeline revision**. Keep a
separate hypothesis and save. Each Trail supplies its own Asset and history.

Choose **No Pipeline** in **Edit Trail** to inspect the Asset without an active
operation. Saved Run history remains visible in the notebook and canvas, including
after restarting the app.

![Asset canvas without a selected Pipeline, displaying its retained ROT13 result and original Run evidence.](assets/desktop/unbound-history.png)

The accepted result remains inspectable independently of the working configuration. [Open full size](assets/desktop/unbound-history.png).

Choose **Fork Pipeline** to create an independently versioned alternative from
the selected revision. The fork records its source; edits to either definition
do not change the other. Use **Edit Pipeline** after the fork to change its
operation or parameters. [Pipeline reference](reference/desktop-pipelines.md)
describes exact commands, revisions and limits.

## Follow an edited Asset

[Edit text or a caption](notebook.md#edit-text-and-captions) in its notebook to
append an Asset revision. Its existing Trail and open canvas show that saved
revision without creating a second Asset or canvas identity. Earlier Runs stay
bound to their original inputs; the result marks an earlier input until another
Run is accepted for the saved content.

## Preserve evidence

Choose **Run cell** to create a PipelineRun, rather than replacing an earlier
execution. [Notebook controls](notebook.md) explain retention and saved reasoning.
Closing a canvas does not cancel its active Run while the app is alive; quitting
the app ends the private engine session. Canvas tabs remain in memory. Saved hypotheses, Pipeline selections and accepted
Runs persist. [Saved state](persistence.md)
explains restart and interruption behavior.

Multi-node Pipeline editing, node connections, branching, joins, output reuse and
parallel execution are not available in this source preview. Their implementation
status lives in the [milestone](https://github.com/cubearc/play/milestone/3).
Use [native Run reference](reference/desktop-runs.md) for implemented execution
semantics and [the domain contract](reference/desktop-domain.md) for full ownership.
