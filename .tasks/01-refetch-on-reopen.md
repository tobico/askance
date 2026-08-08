# 01. Refetch on reopen

## What to build

When the viewer returns to the foreground — the PWA reopened after switching
away or unlocking the phone, or a tab refocused — every active query refetches,
so the human never reads a list that stopped being true while the app was
asleep. The trigger is the document becoming visible again, which is the one
signal that reliably fires on iOS PWA resume.

This overturns the recorded decision that coming back to a tab is not new
information about a Set (see ADR-0005): for an installed PWA, coming back is
precisely when the world has moved. The comment carrying the old rationale is
rewritten to carry the new one.

## Acceptance criteria

- [ ] Reopening the app shows the current pending list without waiting on the
      10-second poll
- [ ] A Set page open when the app resumes catches up the same way
- [ ] The comment that recorded the old no-refetch-on-focus rationale now
      records this decision, pointing at ADR-0005
- [ ] A viewer test drives the visibility trigger and sees the refetch
