# 02. Draft autosave

## What to build

In-progress Answers survive leaving the page. As the human fills in a set view,
the draft is written to `localStorage` keyed by the Set's id; reopening that Set
restores every selected Option, every free-text field and the set-level comment
exactly as they were left.

Deliberately per device and not synced to the server: a phone and a laptop keep
their own drafts. Cross-device draft sync was considered and cut as scope creep.

The draft is cleared once the Set is settled — the Response was accepted, or the
server says the Set was already answered or is no longer there. In all three
cases the draft can never be submitted, so keeping it would only resurface a
stale one on some later visit.

The submit path already snapshots each question's fields into a per-question
`{ label, selected, free_text }` shape; the draft is that list plus the comment,
so it serializes as-is rather than needing a parallel shape. A stored draft that
no longer matches the Set (labels changed, or the JSON will not parse) is
discarded rather than partially applied — the Set as the agent sent it wins.

Two dependency additions in the app crate, both verified as needed during
re-grounding:

- `serde_json` for the draft body
- `web-sys` with the **`Storage`** feature — leptos's own web-sys enables
  `StorageEvent` but not `Storage`, so `local_storage()` will not compile off
  the re-export alone

Storage access has to be browser-only and must not panic the page: a browser
with `localStorage` blocked or full loses drafts and nothing else.

## Acceptance criteria

- [ ] Fill part of a Set, close the tab, reopen it: selections, free text and
      the set-level comment are all restored
- [ ] Submit a Set, then revisit it (or open another Set): no stale draft is
      applied
- [ ] Two different Sets keep independent drafts
- [ ] A draft whose labels no longer match the Set, or whose stored body will
      not parse, is discarded and the Set renders clean
- [ ] Nothing is pre-selected by draft restore that the human did not put there
      — restoring never invents an Answer
- [ ] SSR is unaffected: the server-rendered page is identical with or without
      a draft present, and hydration does not warn
- [ ] Unit tests cover the round-trip of the draft shape and the mismatch
      rejection
