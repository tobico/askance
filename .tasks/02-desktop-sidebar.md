# 02. Desktop sidebar with click-to-jump

## What to build

The ToC itself, on wide viewports. The Set as the browser receives it gains
the Diff's file paths as data (the Diff's HTML stays pre-rendered; the paths
ride alongside it, in Diff order, so the nav is built from the loaded Set
rather than scraped out of the page). From that, a nav mirroring the page top
to bottom: a Preface entry when the Set has one; a clickable "Diff" heading
with one nested entry per file; a clickable "Questions" heading with one
nested entry per top-level Question. It appears on every standing.

Layout: the content column stays exactly where it is (centered, 46rem). Once
the viewport is wide enough to fit both (~64rem), the nav sits sticky in the
otherwise-empty left margin. Narrower than that it is hidden — the mobile
treatment arrives in task 04.

Entries: a Question entry is its label plus the first words of its text on
one truncated line ("Q3 Where should the counter live…"); a file entry is its
path truncated from the left so the filename survives ("…/app/src/set_view.rs").

Clicking an entry scrolls to its anchor — smoothly, instantly under
prefers-reduced-motion — unfolds a folded Diff file before jumping to it, and
records the position in the URL hash via replaceState (no history entry).
Scrolling itself never touches the URL.

## Acceptance criteria

- [ ] On a wide viewport, every Set page shows the sidebar with entries
      mirroring exactly the sections the Set has; the content column does
      not move relative to today, and narrower viewports are unchanged
- [ ] Clicking a file entry whose fold the reader closed unfolds it and
      lands on it; clicking any entry updates the hash via replaceState and
      adds nothing to history
- [ ] Question entries truncate to one line keeping the label and leading
      words; file entries truncate from the left keeping the tail
- [ ] The file paths travel with the Set data in Diff order, covered by a
      server-side test
