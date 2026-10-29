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

## Edit text and captions

Choose **Edit text** in a text Block or **Edit caption** on an image that has a
caption. Change the content, then choose **Save changes** or Cmd/Ctrl+S. The editor
shows its starting revision and UTF-8 byte count; the 1024-byte limit also applies
to edits. **Cancel** discards this edit draft.

Saving keeps the Block and Asset identities and appends a revision. The image and
caption stay in one figure, and editing the caption leaves image bytes unchanged.
Its existing Trail uses the saved revision for the next Run. A Run already in
progress continues with its original input.

An older result remains visible with **Earlier input · r1** when the current
source is r2. Choose **Run cell** to execute the saved content; earlier Runs remain
in history. Saving text does not execute it, and Cmd/Ctrl+Enter inside the editor
does not start a background Run. Cmd/Ctrl+S saves the editor without saving a
separate reasoning draft.

![Caption editor with revision, byte count and explicit Save changes and Cancel controls.](assets/desktop/caption-editor.png)

The caption editor in the macOS source preview. Saving preserves earlier input
revisions and Run evidence. [Open full size](assets/desktop/caption-editor.png).

If another edit was saved first, the editor keeps your draft and reports a
conflict. Copy any text you want to keep, then **Cancel** and reopen the editor to
review the latest saved revision. It never silently overwrites that content.
Unsaved text/caption edits trigger the same **Keep editing** / **Discard draft**
choice as reasoning when closing or quitting; drafts are not crash-recovery data.

## Reorder the notebook

Use the up and down arrows in a Block's heading to move it one position. The
image, caption and inline result move together. **Reasoning** is also an ordered
Block; move it between clues or place it first.

Tab to a move button and press Enter to use the keyboard. After a successful
move, focus follows the Block and its position is announced. Cmd/Ctrl+Enter in a
text or caption Block runs that Block's selected operation. In reasoning, that
shortcut does not run another Block.

Moves save immediately and survive reopening. They preserve Asset identities,
saved revisions, Trails, earlier Runs and unsaved reasoning. If another action
changed the order first, Play shows the latest order and keeps your draft; try
the move again. Reordering is visual organization, not Pipeline scheduling.

![Reasoning moved before an image and caption, with move arrows in each Block heading.](assets/desktop/ordered-notebook.png)

The ordered notebook in the macOS source preview. Image and caption remain one
Block. [Open full size](assets/desktop/ordered-notebook.png).

New content is placed before reasoning when reasoning is last. If you have moved
reasoning elsewhere, new content is appended without moving the existing Blocks.

## Duplicate a clue

Choose the copy icon in a text or image Block's heading. Tab to
**Duplicate First clue** (or the Block's title) and press Enter to use the
keyboard. The saved content is copied immediately after the source, and focus
follows it. An image and its caption stay together.

The copy has a distinct title, such as **First clue (copy)** or
**First clue (copy 2)**. **Copied from First clue · r1** identifies its original
input revision. Use **Edit text** or **Edit caption** on the copy, then choose
**Run cell** to investigate an alternative. The original content and Runs stay
intact. Copies start without Runs; duplicating a Block never runs an operation.

![Original clue and an independently edited copy with separate ROT13 results and the preserved source revision.](assets/desktop/duplicated-blocks.png)

Two clue variants in the macOS source preview. The copy keeps its origin while
using its own saved input. [Open full size](assets/desktop/duplicated-blocks.png).

Copies keep their own Assets and Trails, so editing either version does not edit
the other. Their content starts at r1 even when copied from a later revision.
The source label stays pinned to that original revision. Saved results and
reasoning are not copied into the new Trail.

If the source or notebook changed first, Play shows the latest content and keeps
your reasoning draft. Review it, then duplicate again. Unsaved text in an editor
must be saved or cancelled before copying. Copies count toward the Stage's eight
Blocks and retained storage limit; the button is disabled when the Stage is full.
**Reasoning** remains one Note Block per Stage and has no duplicate action.

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
| Save editor or reasoning | Cmd+S | Ctrl+S |

Windows/Linux worker execution is refused by the current native policy, even
though the UI has keyboard bindings. [Installation](installation.md) states the
platform scope. Image replacement, Block removal and
result export are not available in this preview.
