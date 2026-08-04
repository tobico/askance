# 01. Anchors and section headings

## What to build

Make every section of the Set page addressable, on every standing. The
Preface and the Questions gain quiet visible h2 headings ("Preface",
"Questions"), styled like the Diff's existing one. Id anchors go on the
Preface section, the Diff section, each file's foldable section within the
Diff (stamped during the server-side render, since the Diff arrives as
pre-rendered HTML), and each top-level Question's list item.

Id naming: `preface` and `diff` for the sections; files by position within
the Diff (`diff-1`, `diff-2`, …), which stays unique whatever the paths are;
Questions by their label, lowercased (`q3`, and a Sub-question never needs
one — it scrolls with its parent). A Set is immutable once sent, so
position-derived ids are stable for its lifetime.

Because the ids are in the server-rendered HTML, hash deep-links work at the
end of this task with no script at all: the browser lands on the named
section natively.

## Acceptance criteria

- [ ] Opening a Set URL with `#q3` lands on the Question labelled Q3;
      `#diff-2` lands on the second file; `#preface` on the Preface —
      before hydration
- [ ] The Preface and the Questions carry visible headings, styled like the
      Diff's, on waiting, answered, and archived-unanswered Sets alike
- [ ] A Set with no Preface or no Diff renders no heading and no anchor for
      the missing section, exactly as it renders no section today
- [ ] Server tests cover the ids in the rendered page, and the diff
      renderer's unit tests cover the per-file ids
