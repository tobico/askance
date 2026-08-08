# Adoption docs

Turns the README from a 697-line working document into a shop window: a
developer already running a CLI coding agent can go from finding the repo to a
working, secured, phone-notifying Askance by reading it top to bottom. The
technical depth it currently carries — the from-source quickstart, the viewer
dev loop, the NixOS walkthrough, the proxy timing notes — is not cut but moved
under `docs/`, where it stays reachable without standing between a newcomer and
the install command.

Two things ride along. The README gains an `## Updating` section, which is what
makes the Update Notice banner shipped in stage 05 resolve — it already links
`#updating`, an anchor that does not yet exist. And `examples/skills/` gains two
real, tool-agnostic skills the README quotes, so the asking-and-gating workflow
arrives as files an adopter can paste rather than as prose about them.

Tagging v0.1.0 is deliberately **not** in these tasks. `release.yml` fires on a
`v*` tag and commits `nix/release.json` straight to `main`, so the tag has to sit
on a commit already on `main` — after both this stage's PR and PR #7 merge. Task
04 leaves a go-live checklist instead.

Roadmap stage: [06: Adoption docs and go-live](docs/roadmaps/public-release/06-adoption-docs.md)

## Tasks

- [x] 01: Docs guides extraction — [details](01-docs-guides.md)
- [x] 02: Example skills — [details](02-example-skills.md)
- [x] 03: README rewrite — [details](03-readme-rewrite.md)
- [ ] 04: Cleanup and go-live prep — [details](04-cleanup-and-go-live.md)
