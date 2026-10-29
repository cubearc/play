# Work in a Stage notebook

<a id="export-an-artifact"></a>
<a id="persistence-and-api"></a>
<a id="private-notes-and-artifacts"></a>

Each Exploration has its own local Stages. A Stage combines ordered content
Blocks, cell results and saved reasoning. Choose **New Stage** in the Explorer
to create another local part without importing a Challenge.

Use [Add text or Add image](workspace.md) to add clues. An image caption stays in
the image's figure, with its own input Asset identity. Clicking an Asset opens a
Trail canvas in the same Exploration; [Explorations and Trails](explorations.md)
explains the relationship.

## Run a text cell

Select **ROT13** or the Brainfuck adapter (`esolang brainfuck run`) and choose
**Run cell** on macOS. The operation
runs through Core in a supervised native worker. Progress, cancellation and the
result appear beside that cell, with the same Run visible in its Trail canvas.
There is no shared Output panel. The preview does not execute a multistep graph.

**Keep result in private history** chooses whether the accepted Run stores its
output. When checked, the result survives restart. When cleared, the saved Run
records **output was not retained**; the output cannot be recovered from history.
Brainfuck output is labeled as Base64 bytes, without an implicit text conversion.

Expand **Saved Run & input evidence** to inspect an earlier Run or load older
history. Accepted Runs pin the input revision and Pipeline/Core configuration;
choosing another operation does not rewrite previous evidence. See
[native PipelineRuns](reference/desktop-runs.md) for states and exact retention.

## Save reasoning

Write in **Stage note**, then choose **Save reasoning** or Cmd/Ctrl+S. Wait for
**Saved** before leaving the Stage. Saving appends an immutable reasoning revision;
a conflict preserves the draft rather than silently replacing stored content.
Closing or quitting with unsaved reasoning offers **Keep editing** or
**Discard draft**. Unsaved drafts are not crash-recovery records.

## Keyboard controls

| Action | macOS | Windows/Linux UI |
| --- | --- | --- |
| Toggle Explorer | Cmd+B | Ctrl+B |
| Open command palette | Cmd+Shift+P | Ctrl+Shift+P |
| Run active text cell | Cmd+Enter | Ctrl+Enter |
| Save reasoning | Cmd+S | Ctrl+S |

Windows/Linux worker execution is refused by the current native policy, even
though the UI has keyboard bindings. [Installation](installation.md) states the
platform scope. Blocks currently support creation and ordered display; a general
content editor, drag reordering and result export are not available in this preview.
