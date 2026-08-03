# 03. Answering conveniences

## Goal

The daily-driver polish: one tap accepts all recommendations, half-finished
answers survive a page close, dead agents are visible at a glance, and
answered or orphaned Sets live in a browsable Archive.

## Decisions in force

- **Accept-all is an explicit button** (grilling session Q10) — fills every
  *unanswered* Question with its ★ Recommendation; already-made Answers are
  untouched; individual answers can still be changed before submit. Nothing
  is ever pre-selected on load — a sleepy thumb-tap must not approve unread
  decisions. This is the UI realization of the grammar's `*` reply.
- **Drafts autosave to localStorage, per device** — deliberately not synced
  server-side; cross-device draft sync was considered and deferred as scope
  creep.
- **Liveness is display-only** ([ADR-0001](../../adr/0001-blocking-cli-for-agent-integration.md)
  consequences) — the server knows whether a long-poll is currently held for
  a Set and badges it “agent waiting” vs “agent disconnected”. Disconnection
  never auto-withdraws a Set: the CLI reconnects through transient drops, so
  only a human may archive an orphaned Set.
- **Archive is permanent** (grilling session Q8a) — answered Sets are a
  browsable decision log, never deleted. Manual archiving of an orphaned
  (never-answered, agent-dead) Set moves it to the Archive too, distinguishable
  from answered ones.

## Proposed tasks (provisional)

1. **Accept-all recommendations button** —
   - Fills only unanswered Questions with their ★ Option
   - A set with no Recommendations shows no button
2. **Draft autosave** — persist in-progress Answers to localStorage, restore
   on reopen, clear on submit.
   - Close tab mid-answer, reopen: state restored
   - After submit, no stale draft resurfaces
3. **Liveness tracking + badge** — server tracks held long-polls per Set; UI
   badges each pending Set.
   - Kill the CLI: badge flips to “agent disconnected” (after the reconnect
     grace window); restart it: flips back
4. **Archive** — archive view for answered Sets, manual archive action for
   orphaned ones.
   - Submitted sets appear in Archive with their Response readable
   - Manually archived (unanswered) sets are visibly distinct

## Re-verify at start

- How stage 01 implemented the long-poll (connection lifetime, reconnect
  interval) — liveness detection needs a grace window tuned to it.
- Whether stage 02 kept form state client-side in a shape localStorage can
  serialize directly.
- Assumes stage 02 landed (set view + submit exist).
