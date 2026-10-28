# Visual Lab implementation and verification

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

## Raw stream transforms

The private ChaCha, XChaCha, Salsa and educational Vernam operations accept
explicit named byte inputs through the existing Lab execution API. Keys and pads
are per-run inputs; output retention requires explicit choice. The byte viewer
supports exact preview and export. Query installed Core contracts for exact
profiles, nonce arrays, counter widths and limits. No pipeline adapter is enabled.
