# Inspect an image in its canvas

<a id="visual-lab"></a>

[Add a PNG with its caption](workspace.md), then click the image in the Stage
notebook. Its Trail canvas opens in the same Exploration and shows the imported
image with zoom controls. The original source file stays unchanged.

Click the caption to open its own text canvas. Its ROT13 or Brainfuck operation
uses the caption Asset, not the image pixels. The caption still appears with the
image in the notebook.

This preview has no general image-operation editor, pixel-spiral pipeline,
artifact comparison or arbitrary media viewer. A Core catalog entry does not
establish a native Play adapter. The current native execution behavior is in
[PipelineRuns](reference/desktop-runs.md); the canonical graph model is defined
in [the domain contract](reference/desktop-domain.md).
