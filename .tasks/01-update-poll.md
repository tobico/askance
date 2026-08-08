# 01. The update poll and what it answers

## What to build

The server learns whether a newer Askance has been released, and says so on
the viewer's namespace.

A background task asks GitHub for the repository's latest release — once at
startup, then once a day — and compares the tag against the version compiled
into the binary. What it concluded is held in memory; nothing is written to
the store, and a poll that fails leaves the last verdict standing and is
retried next cycle.

`GET /api/ui/update` hands the verdict to the viewer, carrying the newer
version's number so the banner can name it. Its type is written out through
`crates/render`'s TypeScript export like every other payload the viewer reads
— the list of roots there is the viewer's wire surface, so a new payload
belongs on it.

`--no-update-check` / `ASKANCE_NO_UPDATE_CHECK` on the server config turns the
whole thing off: the task never starts and no request is ever made. It sits
alongside `--database` and `--listen` in the same style.

Decisions this task is working under:

- **`releases/latest`**, which excludes pre-releases. `v0.1.0-rc.1` is a
  rehearsal tag marked as one, so today the endpoint 404s — that is the
  behaviour wanted, and a 404 means no news rather than an error to report.
  It starts answering when stage 06 tags `v0.1.0`.
- **The `semver` crate** does the comparison. A tag that will not parse is no
  news, same as a 404.
- **Where GitHub lives is an internal parameter**, not a flag: `run` passes
  the compiled-in address, and a test passes a server it stood up itself.
  Nothing about it reaches the CLI's help.

## Acceptance criteria

- [ ] Against an in-process server naming a version above the crate's,
      `GET /api/ui/update` says an update exists and names that version
- [ ] Against one naming the crate's own version, or an older one, it says
      there is nothing to update to
- [ ] A 404, a connection failure, or a tag that will not parse leaves the
      endpoint saying nothing is available, and is not fatal to the server
- [ ] With `ASKANCE_NO_UPDATE_CHECK` set, no request is ever made — proven by
      a server that records every request it receives — and the endpoint still
      answers, saying nothing is available
- [ ] The poll happens at startup rather than only after the first day
- [ ] `web/src/api/types.ts` gains the payload's type from `cargo test`, not by
      hand
