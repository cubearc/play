# Candidates and Solutions

Choose a private answer for a local Stage without changing the evidence that
produced it. A **Candidate** records an exact Asset revision or a named output
from a completed, retained Run. Later edits and executions leave that selection
intact.

## Choose an answer

1. Open an Asset canvas and choose **Choose as candidate**, or run a notebook
   cell and choose **Choose as candidate** beside its output.
2. Open the Stage notebook. **Candidate** shows the selected content and its
   origin, marked **Private · Unverified**.
3. Choose another answer when your investigation changes. Use **Selection** to
   inspect an earlier Candidate revision. Looking at history does not replace
   the current answer.
4. For a selected Run output, choose **Open source Run** to inspect the original
   inputs, Pipeline revision and node attempts.

![Stage notebook with a selected ROT13 result and Candidate revision history](assets/desktop/stage-candidate.png)

For example, select `Gur xrl.` as a Candidate, edit its working Asset to
`Svefg pyhr.`, and run **ROT13**. The first Candidate still contains `Gur xrl.`;
selecting the Run output creates another Candidate revision containing
`First clue.`. Both remain available after reopening the workspace.

An image and its caption remain grouped in the notebook. Open the image or
caption canvas to choose that specific Asset. Selecting a file preserves its
bytes; the Candidate panel labels its displayed text or hex as a preview.

<a id="evidence-and-limits"></a>

## Evidence and limits

Each Stage has one Candidate with up to 32 immutable selections. Different
Stages and Explorations keep separate histories, including independent imports
of the same Challenge. Editing or archiving a Block does not replace a selected
Asset revision. No Candidate revision is overwritten or silently deleted.

Run selections retain their named output type, including numbers and structured
values. Play does not convert them to text implicitly. Use explicit Pipeline
operations to select a field, convert a value or combine results before choosing
an answer. Outputs that were not retained cannot become Candidates. Unavailable
evidence is shown as a gap while preserving its selected reference.

If another action changes the Candidate first, Play reloads its current revision
and asks you to review and choose again. It never silently replaces that newer
selection. Repeating an accepted request does not create another revision.

<a id="private-solutions"></a>

## Solutions and official outcomes

A **Solution** will preserve a chosen result, explanation and only its contributing
evidence path. Solution snapshots and walkthrough export are not yet available in
the Electron preview. Candidate selection does not imply those capabilities.

Candidates are private and unverified. Neither a successful Run, an integrity
hash nor available evidence establishes that a Challenge answer is correct.
Official submission, evaluation and publication belong to the separate Cloud
project and are not performed by **Choose as candidate**.

<a id="local-api"></a>

The [storage reference](reference/desktop-storage.md#stage-candidates) documents
exact ownership, revision and IPC rules. Use [Run history](reference/desktop-runs.md)
to inspect execution evidence and [the notebook](notebook.md) to keep reasoning.
The remaining Solution workflow is tracked in
[#50](https://github.com/cubearc/play/issues/50).
