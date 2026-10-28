# Browse and inspect local files

The workbench uses the full window width, with the workspace name and connection
state in its compact header. The sidebar groups Files, Explorations and Notes under
Investigate, Artifacts and Solutions under Library, and Local challenges and Terminal
under Tools. Narrow windows use a horizontal section bar. Section changes retain
mounted editors and drafts. Select the investigation name or **Rename** to reveal its
name editor; navigation-save status remains in the compact investigation toolbar.

Open a directory with `cube play [path]`. **Source files** navigates its read-only
contents; **Imported files** shows copies explicitly saved locally. Navigation is
shallow. Direct-file launches retain siblings. Exactly one compatible file in a
complete listing opens automatically; incomplete listings never claim uniqueness.

Select a file for a preview. Core validates supported PNG/JPEG/GIF/BMP/WebP rasters
before their original bytes are served. Rejected images retain metadata and byte
inspection. MP4/MOV/WebM container metadata also comes from Core. Audio/video
playback depends on browser codecs and starts muted; enable sound in the controls.
Unsupported playback retains available metadata and hexadecimal/ASCII inspection.

Filename categories are extension hints. Core header signatures are separate
evidence, never proof of structural validity. Unknown files remain selectable.
HTML, SVG and scripts are inert UTF-8 text, never embedded documents. Unsupported
text/control bytes use byte inspection without silently converting original data.

## Import explicitly

Choose a file, review **Save as**, then select **Import into workspace**. Selection
alone sends no bytes. Imports go into private data-volume storage, never the source.
Existing names are rejected; choose another name rather than overwriting them.
Imports survive restart/recreation with stable SQLite references. See
[local persistence](persistence.md) for saved navigation and missing-file recovery.

An interrupted import can have completed after its commit point; refresh before retrying. See [import publication](reference/workspace.md#atomic-import-publication) for recovery details.

## Limits and recovery

Listings and previews are bounded and identify incomplete results. Refresh after host renames or removals and narrow large directories when needed. The [workspace reference](reference/workspace.md#limits-and-recovery) lists exact quotas, supported paths, authorization and storage requirements.
