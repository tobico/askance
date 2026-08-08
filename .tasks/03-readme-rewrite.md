# 03. README rewrite

## What to build

The README becomes adoption-first, written for a developer already using a CLI
coding agent who has just found the repo. The agreed outline, in order:

1. **What Askance is** — the problem it solves, short.
2. **Installing the binary** — a curl one-liner fetching to `~/.local/bin`,
   built on the `releases/latest/download` URL so the README never names a
   version and never goes stale. The nix path alongside it
   (`nix run github:tobico/askance`, and the flake input for a persistent
   install).
3. **Setting up the server** — `askance serve`, with pointers out to the
   systemd example and the NixOS page from task 01.
4. **Configuring your agent** — what goes in `CLAUDE.md`, or whatever file the
   adopter's harness reads at the start of a session.
5. **Skills** — asking questions, and an acceptance gate before commit, quoting
   the two files written in task 02.
6. **Securing access** — Tailscale: tailnet-only, `tailscale serve`, and
   **never funnel**. Depth lives in the guide from task 01.
7. **Updating** — the section the Update Notice banner links to.

Section 7 is load-bearing beyond the README: `web/src/update/UpdateNotice.tsx`
already links `https://github.com/tobico/askance#updating`, and that anchor does
not exist today. Whatever heading is written has to produce exactly that anchor.

Everything factual in the new README has to match what actually shipped in the
earlier stages, so check rather than transcribe from the brief. The reference
material sections the README keeps — the Guide, Configuration, the wire format,
the API — stay, but they sit after the adoption path rather than interleaved
with it.

Watch the existing cross-link in the Guide's "Installing it" text, which points
at `#deployment` to explain how `askance` reaches a user's `PATH`. That anchor
moves in this rewrite and the link has to follow it.

## Acceptance criteria

- [ ] A reader can go from the top of the README to a working, secured,
      phone-notifying Askance without leaving the page except by choice
- [ ] `#updating` resolves — the anchor the stage 05 banner links to exists,
      and the section actually tells someone how to update
- [ ] The install one-liner runs verbatim and produces a working binary, using
      `releases/latest/download` rather than any pinned version
- [ ] Every claim matches the landed code: the verbs (`serve`, `ask`, `guide`),
      the asset names, the flake attributes, and `ASKANCE_NO_UPDATE_CHECK`
- [ ] The skills sections quote the real files from `examples/skills/`, and the
      quotes match those files
- [ ] Tailscale guidance says tailnet-only and rules out funnel explicitly
- [ ] Every link resolves, including the Guide's repointed "Installing it"
      cross-link and every link into the task 01 guides
