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

Click a text Asset, image, caption or file in a Stage notebook. Play opens a **Trail
canvas** tab for that Asset in the same Exploration. A caption's canvas shows its
exact bound Asset revision, selected reusable Pipeline and accepted Run history.
The image's canvas supports inspection and zoom. A file's canvas shows an inert
text/hex preview, byte count and SHA-256 digest. Results stay with the cell or
canvas that produced them.


![Caption Trail canvas with its exact source Asset, reusable Pipeline and completed ROT13 Run.](assets/desktop/caption-canvas.png)

Opening the caption creates its own canvas tab inside the same Exploration. [Open full size](assets/desktop/caption-canvas.png).

A Trail holds investigation context and input bindings. A Pipeline is the
Workspace-owned reusable operation definition. The current source preview
creates/reuses single-node definitions from Core's native adapters. Different
Explorations can bind their own text to the same definition without sharing notes
or Run evidence. Selecting another operation freezes a separate definition for
the accepted Run.

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
the app ends the private engine session. Canvas tabs and operation-selection
drafts remain in memory, while accepted Runs are stored. [Saved state](persistence.md)
explains restart and interruption behavior.

General Pipeline editing, node connections, branching, joins, output reuse and
parallel execution are not available in this source preview. Their implementation
status lives in the [milestone](https://github.com/cubearc/play/milestone/3).
Use [native Run reference](reference/desktop-runs.md) for implemented execution
semantics and [the domain contract](reference/desktop-domain.md) for full ownership.
