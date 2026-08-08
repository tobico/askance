# 02. The Update Notice banner

## What to build

The pending page draws the Update Notice: a banner above the list, naming the
newer version and linking the instructions for updating, shown only when the
server says there is one.

It asks `/api/ui/update` on its own, separately from the pending list, and at
its own cadence — the list refetches every ten seconds because a Set can
arrive at any moment, and a release cannot. The server polls daily and holds
the verdict, so the page has nothing to gain by asking often.

The Notice informs and never installs (CONTEXT.md): the link is the whole of
what it offers, and there is nothing to click that changes the server. It is
not dismissible — it stands until the server is updated, at which point it
stops being drawn on its own.

The link points at `https://github.com/tobico/askance#updating`. That anchor
is stage 06's to write and does not exist yet; stage 06 re-verifies it.

## Acceptance criteria

- [ ] The Notice appears above the pending list when the server says an update
      exists, and names the version
- [ ] It links the updating instructions, and offers nothing that installs
- [ ] Nothing is drawn when the server says there is no update, when the
      request fails, or while it is still in flight — a page that cannot reach
      the endpoint shows the pending list exactly as before
- [ ] vitest covers the shown and not-shown cases
- [ ] It reads as part of the page on a phone-width window, and does not push
      the list out of the reading column
