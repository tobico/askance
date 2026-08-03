# 04. Diff viewer

## What to build

Render the attached Diff in the set view so code-approval questions are
reviewable without leaving the page. Sets without a Diff simply omit the
section.

Stage 01 stores the Diff as one raw unified-diff string on the Set (all
uncommitted changes including untracked files, binary contents omitted), so
the viewer parses unified diff text itself: split per file, show each file
under its own header with hunk structure and added/removed/context line
colouring.

Token-level syntax highlighting is in scope (decided at planning): highlight
server-side (syntect or similar), keyed off each file's extension, on top of
the +/- line colouring. Files the highlighter doesn't recognise fall back to
plain +/- colouring.

Rendering happens server-side during SSR — the client doesn't ship a diff
parser or highlighter. Long diffs should stay navigable on a phone: per-file
sections, horizontal scrolling contained within the diff rather than the
page.

## Acceptance criteria

- [ ] A Set with a Diff spanning modified + untracked files renders per-file
      sections with correct added/removed line colouring
- [ ] Recognised file types get token-level syntax highlighting;
      unrecognised ones degrade to plain +/- colouring
- [ ] A Set without a Diff shows no diff section at all
- [ ] A large Diff doesn't break the page layout (contained scrolling,
      per-file sections)
