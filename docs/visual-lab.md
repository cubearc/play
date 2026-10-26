# Visual Lab

Select an image in the file explorer, choose **Open visual Lab**, then **Decode
image in Core**. Opening or browsing a directory does not run this analysis.
Unsupported media keeps its existing metadata and byte inspector. Core decoding
errors and the 4 MiB input limit appear with a retry action; sources stay read-only.

The original Core-decoded raster and active image appear side by side. Browser
color management is display only. Core reports the decoded format, sample model,
bit depth and origin; PNG transport does not imply preservation of every original
metadata chunk. Pixel coordinates are zero-based from the top left of that raster.

- Use **Pixel X/Y**, then **Read pixel**, for exact native sample values. Click the
  256 × 256 canvas to choose a coordinate; number inputs provide keyboard access.
  Zoom and grid change presentation only. No canvas sample reads are used.
- Choose an analysis channel and **Compute histogram**. ECharts plots exact Core
  counts without aggregation or approximation. **Histogram values** exposes the
  same tuples in a semantic table. Missing bins are omitted.
- **Extract channel** generates a Core grayscale preview and a structured result.
  Indexed rasters expose indices until **Expand palette to RGBA** is explicitly
  selected; expansion retains palette transparency through Core.
- Invert, grayscale, rotate 90 degrees or mirror horizontally create independent
  image results. **Compare with original** requests exact Core differences;
  incompatible dimensions/models fail clearly instead of being resampled.
- Select a generated result to inspect its type, effective parameters, input
  references, versions and digests. Selecting an image makes it the next operation
  input. Structured results remain inert JSON. A transient inspection has
  provenance but cannot be reused as an artifact.

Only one operation runs at a time. **Cancel operation** requests worker termination;
leaving the file also requests cancellation. Publication already in progress may
finish, as described in the [API](lab-api.md). Errors preserve the existing original
and results; retry explicitly. Managed results survive restarts within the existing
64-result/64-MiB quota. This initial result list is session-local; reopening named
artifacts/history belongs to the notes and artifacts capability. No results are
submitted, uploaded or used as challenge answers automatically.

The UI uses React hooks and lazy loads visual controls and the ECharts chart.
Canvas renders a bounded viewport; Core supplies every histogram, sample, channel
and transformation. ECharts uses modular imports, a resize observer and disposal
on unmount. See the official [module guide](https://echarts.apache.org/handbook/en/basics/import/)
and [chart lifecycle guidance](https://echarts.apache.org/handbook/en/concepts/chart-size/).

Run the frontend coverage suite for boundary/component tests and Playwright for
real Rust/Core desktop/mobile workflows. The Docker runtime smoke suite also runs
those Lab flows inside the actual local container. Original RGB and indexed PNG
fixtures have known counts, palette values and transparency; malformed and oversized
inputs are generated only in temporary test workspaces. Tests check source bytes,
no external requests, result selection, keyboard controls, accessibility and overflow.
