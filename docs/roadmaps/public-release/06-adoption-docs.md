# 06. Adoption docs and go-live

## Goal

A developer already using a CLI coding agent (Claude Code, opencode) can go
from finding the repo to a working, secured, phone-notifying Askance by
reading the README top to bottom — and v0.1.0 is tagged, making the release
real.

## Decisions in force

- **The README is adoption-first; the technical depth moves to docs guides**
  (chosen over one long README and over cutting the material). The current
  735-line README's quickstart-from-source, proxy timing notes and NixOS
  walkthrough are worth keeping — just not in the shop window.
- **Agreed README outline** (from the grilling session, in order): what
  Askance is; installing the binary (the curl one-liner to `~/.local/bin`
  using the `releases/latest/download` URL — chosen so the README never goes
  stale — plus the nix path); setting up the server (`askance serve`, with a
  pointer to the systemd example and the NixOS page); configuring your
  CLAUDE.md or other agent; skills — asking questions and an acceptance gate
  before commit, quoting the two example skills; securing access and push
  with Tailscale (tailnet-only, `tailscale serve`, never funnel); updating
  (the anchor stage 05's banner links).
- **The example skills are real files under `examples/`, quoted in the
  README** — a grilling skill and an acceptance-gate skill, **tool-agnostic
  markdown** (the user chose this over Claude-specific SKILL.md format: the
  target adopter may run any CLI agent). Keep each one short enough to read
  in the README excerpt.
- **Non-NixOS daemonization gets a short systemd unit example in docs**; the
  NixOS module walkthrough becomes its own docs page with a README mention.
- **Stale-content cleanup flagged by the history audit**: the "give nix a
  GitHub token / Askance is a private repository" README section goes
  entirely; `docs/agents/git-workflow.md` drops "(private)" and the
  machine-specific SSH note.
- Go-live is tagging **v0.1.0** once the docs land (the repo went public
  back at stage 03).

## Proposed tasks (provisional)

1. **Guides extraction** — development (from-source quickstart, viewer dev
   loop), deployment (NixOS page, systemd example), phone/Tailscale depth
   moved under `docs/`, README links surviving. Accepts: no information
   lost, links resolve.
2. **README rewrite** — the outline above, written for the target adopter.
   Accepts: install command works verbatim on a clean machine; every claim
   matches the landed stages (verb, asset names, env var).
3. **Example skills** — `examples/` gains the grilling and acceptance-gate
   skills; README quotes them. Accepts: each skill usable by pasting into an
   agent's instructions; the gate example follows the Guide's gates topic.
4. **Cleanup and tag** — audit-flagged staleness fixed, v0.1.0 tagged, the
   release verified end to end (curl install, nix run, Update Notice sees
   the release).

## Re-verify at start

- Everything earlier landed as briefed: the verb, asset names, flake
  attributes, the opt-out env var's actual name, the banner's anchor.
- The audit-flagged sections still read as they did (README ~line 290,
  `docs/agents/git-workflow.md` notes).
- Whether the Guide's wording (`askance guide`) needs any touch to match the
  new install story.
