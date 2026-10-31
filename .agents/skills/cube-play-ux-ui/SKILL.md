---
name: cube-play-ux-ui
description: Design, implement, critique, and refine purposeful UX/UI for Cube and Cube Play. Use for frontend feature implementation, user-visible bug fixes, interface reviews, Play workbench screens, notebooks, asset viewers, operation discovery, pipeline editors, run results, challenge authoring, screenshots, and requests to improve usability, visual hierarchy, density, accessibility, or remove generic AI-generated UI. Ground decisions in the current CubeArc product, native components, real capabilities, and investigation workflows. Do not use for unrelated products or backend-only algorithm work.
---

# Cube/Play UX/UI Anti-Slop

Resolve repository paths from the Play repository root. Apply this workflow throughout relevant feature work, fixes, and reviews. Respect review-only and planning-only requests; those scopes do not authorize implementation.

Build an investigation environment that makes evidence easy to inspect, operations easy to apply, and results easy to trust. Give every control, container, color, and transition a task-related purpose.

Treat anti-slop as a quality standard: deliberate composition, clear language, useful density, coherent interactions, and truthful state. Preserve character and craft. Avoid replacing one generic template with an arbitrary monochrome or brutalist template.

## 1. Establish the task and the active surface

Before changing UI:

1. Read the repository's `AGENTS.md`, relevant product/domain contracts, and existing UI conventions. Inspect the actual entry point, affected components, tokens, and nearby interactions.
2. Identify the user, selected object, intended action, expected result, and likely failure. Capture the task in brief working notes. Create a separate design document only when the task needs one.
3. Separate observed implementation, intended product behavior, and proposals. Verify capabilities against current code, Core descriptors, adapters, and runtime support. A catalog entry or roadmap item does not prove executability. Flag conflicting documentation.
4. Reuse the existing stack and visual system. Introduce a dependency or new pattern only when it resolves a demonstrated gap. Keep changes proportional to the request.
5. For screenshot-only work, distinguish visible problems from interaction assumptions. State what needs runtime verification. Do not invent hidden behavior or claim to have tested it.

For `cubearc/play`, inspect `docs/architecture.md`, `docs/reference/desktop-domain.md`, and `docs/explorations.md` when relevant. The native renderer currently lives in `frontend/desktop-renderer-src/`; start with `Workbench.tsx`, `Notebook.tsx`, `AssetCanvas.tsx`, `PipelineEditor.tsx`, `PipelineGraph.tsx`, `InlineResult.tsx`, `theme.css`, and `style.css`. Verify these paths and their role before editing; the product has one native renderer.

Use the existing React/VSCode Elements/Codicons system and semantic theme variables when working on that renderer. Treat these as discovery anchors, not a permanent dependency or feature inventory.

## 2. Keep the product model legible

Keep Cube responsible for canonical operations and execution contracts. Keep Play focused on exploration, composition, inspection, and retained evidence. Keep local exploration usable without publishing a Challenge.

Preserve the current domain vocabulary:

| Concept | UX responsibility |
| --- | --- |
| Workspace / Exploration | Show where work belongs and which independent investigation is active. |
| Stage / Block | Organize ordered notebook content without imposing a linear solving process. |
| Asset | Identify the selected content and revision used as input. |
| Trail | Keep a hypothesis, its bindings, and investigation history together. |
| Pipeline | Present a reusable graph definition and its selected revision. |
| Run / node attempt | Show an actual invocation, its execution state, and resulting evidence. |
| Candidate / Solution | Represent a selected answer and supported evidence only where the workflow exists. |
| Challenge edition | Identify published material and service-confirmed participation separately from local work. |

Use familiar action labels in the UI and expose technical identities in inspectable details. Do not rename a Pipeline graph to Exploration or silently conflate drafts, configuration revisions, and executed results. Expose only concepts needed for the current task.

## 3. Compose around the next useful action

Maintain orientation: active Exploration, Stage or Asset, selection, execution state, and available next action. Establish one dominant action per task region while keeping useful secondary actions accessible.

Choose the container from the work:

- Use a notebook for ordered content and reasoning, a list or tree for navigation, a table for comparison, and a canvas for spatial relationships. Use cards for independent previews or choices that benefit from them.
- Keep source, relevant controls, and result within an understandable context. Put operation results beside their producing cell or canvas, with a route back to the exact Run. Avoid forcing users to search a disconnected output area.
- Keep selection, opening, editing, and execution distinct. Selecting an Asset must not execute an operation. Show its compatible operations and explain unavailable actions when that information helps.
- Preserve context when opening inspectors or editors. Use dialogs for bounded decisions; keep sustained editing in a surface where inputs and evidence remain available.
- Support both discovery and fluent repeated work. Provide visible actions first, then search, contextual menus, command palettes, and accurate keyboard hints where supported.
- Make an empty folder a valid workspace. Give an empty state a short explanation and a useful action supported by the current product. Never create fake activity to populate it.

### Apply these patterns when relevant

**Assets and viewers.** Show meaningful previews, content type, and selection. Keep full values available for copying or inspection when display values are shortened. Keep an image and its caption visually related while preserving their separate Asset identities; processing caption text must not imply OCR. Label representations such as hex, Base64, text encoding, units, and coordinate origin when they affect interpretation. Never alter underlying bytes or copy values for visual polish.

**Operations and pipelines.** Surface input requirements, named ports, parameters, and output selection. Explain incompatible connections at the point of action. Show what a Run will target and which dependencies are involved. Keep graph layout, selected configuration, input bindings, and historical execution distinguishable. Provide a form/list alternative for essential graph actions and an alternative to dragging. Do not depict sequential execution as parallel or invent scheduling behavior.

**Results and evidence.** Make the result readable first, then reveal parameters, revisions, node attempts, and provenance on demand. Support comparison when the task requires it. Distinguish current configuration from the configuration that produced an earlier result. Keep original inputs and past evidence intact. Saving an output as a Block must preserve its source Run and named output without silently executing again.

**Challenge authoring and participation.** Keep authoring, previewing as a participant, solving locally, submitting, and publishing explicit when implemented. Show prerequisites, stage availability, optional deadlines, and rewards only from real data. Preserve intentional hidden information and spoiler boundaries. Make preview, publication contents, and the chosen revision understandable. Treat official acceptance and rewards as service-confirmed outcomes. Do not fabricate Cloud integration or require it for local work.

## 4. Give the interface a coherent visual language

For the existing native workbench, build on charcoal surfaces, restrained separators, system sans typography, monospace evidence, and the established action/focus accent. Improve weak contrast, tiny metadata, inconsistent spacing, and hard-coded colors rather than preserving defects for consistency. Follow an explicit user-requested art direction when it changes this baseline.

- Use semantic tokens for surfaces, text, borders, selection, focus, and status. Keep accent color purposeful. Distinguish selection, hover, focus, running, warning, and failure through more than hue.
- Establish hierarchy with alignment, proximity, type size/weight, and whitespace before adding containers. Give the investigation content the largest useful share of the viewport.
- Use readable prose typography; reserve monospace for commands, hashes, bytes, code, and aligned values. Do not shrink essential labels to make a dense layout fit.
- Reuse a small, coherent spacing and radius scale. Give related elements shared edges and consistent rhythm. Prefer a few meaningful surfaces over repeated nested borders.
- Keep icon style and size consistent. Give unfamiliar or consequential controls visible labels. Provide accessible names for every icon button.
- Use motion only to explain state, continuity, or direct manipulation. Respect reduced motion; avoid decorative loops and animated backgrounds.
- Preserve useful visual texture in the evidence itself: images, waveforms, graphs, matrices, and diffs. Keep decoration subordinate to that content.

### Remove common sources of slop

| When you find | Replace or justify it with |
| --- | --- |
| A welcome hero or large slogan above the active workspace | A compact title, orientation, and immediately usable work area. |
| KPI tiles, charts, or counts without a user decision | Actual recent work, actionable state, or no section. |
| Every label and paragraph wrapped in a rounded card | A layout based on relationships, aligned rows, separators, and purposeful grouping. |
| Gradients, glass, glows, neon borders, or decorative grids | Existing tokens and a clear functional reason for any retained treatment. |
| Many equally prominent buttons or badges | A hierarchy based on the current task and meaningful state distinctions. |
| Repeated headings, breadcrumbs, and descriptions saying the same thing | One clear location cue and context only where needed. |
| Generic copy such as “Unlock insights” or “Something went wrong” | A concrete action or a specific failure with a recovery step. |
| Raw schemas or JSON as the only normal workflow | A task-oriented control or viewer, with raw inspection available when useful. |
| A large blank canvas with no starting affordance | A supported first action, input, or explicit example the user chooses to load. |

Apply these as contextual checks. Retain a treatment when it serves the product, content, or user-requested direction. Do not ban a font, card, gradient, or framework solely because it is common.

## 5. Design truthful states and recovery

Cover the states reachable in the affected workflow: empty, loading, ready, selected, editing, queued, running, completed, failed, canceled, unavailable, and conflicted. Use the engine's actual state model; do not add imaginary states or transitions.

- Keep action scope visible: the selected Asset, node, cell, or Exploration. Scope keyboard shortcuts and async responses to that context; typing or saving elsewhere must not trigger a Run.
- Acknowledge pending actions, prevent accidental duplicate submission, and report the accepted outcome. Show progress only when measurable; label indeterminate work honestly.
- Preserve drafts, selection, useful scroll position, and evidence across recoverable failures and context switches. Surface save conflicts without discarding local edits or retargeting writes.
- Keep actionable errors near the relevant input or result. Use persistent messages for failures that require attention; use toasts for lightweight confirmations. Name the problem, affected object, and available recovery.
- Clearly identify an earlier-input result, missing source, unavailable output, or output that was not retained. Derive provenance labels from recorded bindings and revisions; show unknown provenance when records are missing. Do not infer an input binding from timestamps. Distinguish a valid empty output from missing evidence; do not render missing data as an empty successful result.
- Follow actual cancellation and retry semantics. Distinguish a cancellation request from a confirmed cancellation when the engine does. A retry must not relabel historical evidence as a new success.
- Keep ordinary reversible actions lightweight. Use undo or archive where supported. Give destructive actions explicit scope and confirmation when justified by actual loss.
- Label local saves, retained evidence, exports, and remote publication accurately. Preserve native boundaries and user control over private content; surface these details only when they affect the user's decision.

Use plain English: “Run pipeline,” “Save as block,” “Open source Run,” and “Choose an input.” Follow existing product capitalization. Avoid promotional filler, excessive helper text, and invented success claims.

## 6. Preserve access and useful density

- Provide semantic controls, associated labels, logical focus order, visible focus, and keyboard access to essential actions. Restore focus after dismissing an overlay. Keep editor and terminal shortcuts scoped.
- Give drag, hover, and pointer-only interactions an accessible equivalent. Make key graph relationships inspectable without relying solely on the canvas.
- Meet WCAG 2.2 AA text contrast: at least 4.5:1 for normal text and 3:1 for qualifying large text. Check real foreground/background pairs, including metadata and focused or selected states.
- Meet the 24 by 24 CSS pixel minimum target criterion or its documented exceptions; use larger targets when practical. Visual icon size need not equal hit-area size.
- Make surrounding navigation, prose, forms, and inspectors reflow at narrow widths and high zoom. Keep necessary two-dimensional scrolling local to graphs, images, and data regions that need it. Do not use the canvas exception to excuse an overflowing application shell.
- Collapse or move secondary panels before shrinking text or hiding essential actions. Respect supported native minimum window sizes, zoom, reduced motion, and existing theme support. Do not invent a mobile shell or a second theme outside scope.

Use the W3C guidance for [text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html), [target size](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html), and [reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html). These checks do not by themselves establish full accessibility conformance.

## 7. Implement, inspect, and finish

For an implementation request, make the authorized changes. Reuse existing components, typed contracts, and Core metadata. Keep domain algorithms, authority checks, and scheduling out of presentation code. Avoid duplicate operation catalogs, one-off CSS systems, and unrelated redesigns.

For a review request, prioritize task blockers, state/evidence errors, orientation, readability/accessibility, then cosmetic issues. Describe each finding as **observed problem → user consequence → concrete change**. Mark inferred behavior. Do not turn a visual review into an unsolicited backend rewrite.

Use this completion check on the affected surface:

1. Can the user identify their current context, selected object, and next action?
2. Can they complete the main task with realistic content and recover from a relevant failure?
3. Are available actions backed by actual capabilities and correctly scoped?
4. Can they tell input, configuration, Run, result, and provenance apart when needed?
5. Do long names, many items, large outputs, and empty states remain usable?
6. Do keyboard use, zoom/reflow, focus, contrast, and the supported window sizes work?
7. Does the result fit the product's visual system, with each prominent element earning its place?

Inspect the rendered result when the environment supports it. Exercise the changed primary interaction and a relevant failure or recovery path. Follow `CONTRIBUTING.md` and `docs/reference/validation.md` for applicable checks. Preserve existing CI and coverage gates; add tests only for substantive new behavior or concrete regression risk. A screenshot is evidence of appearance, not working interactions. If runtime access is unavailable, identify the verification gap explicitly.

Fix material issues before completing an implementation. Do not average correctness or accessibility defects into a favorable aesthetic score. Report what changed, why it improves the task, what was actually checked, and any remaining limitation. Keep the handoff concise and human-readable.
