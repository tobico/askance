# 07. Archive and Liveness

## What to build

The archive page: the permanent, browsable list of settled Sets, loading
once (no polling) and linking through to each Set's record. Liveness — agent
waiting versus agent disconnected — displayed wherever the Leptos viewer
shows it today, fed by the waits the server holds; display state only, never
causing automatic withdrawal. Finish with pending-list parity polish:
ordering, and anything the walking skeleton left rough.

## Acceptance criteria

- [ ] The archive page lists settled Sets and links to their records
- [ ] Liveness display matches the waits actually held on the server
- [ ] Pending and archive behaviour assertions from the old page tests are
      ported to vitest and green
