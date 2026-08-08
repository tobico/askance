# 03. README loses the private-repo token

## What to build

The README still opens its NixOS walkthrough by telling the reader Askance is a
private repository and walking them through giving nix a GitHub access token —
three variations of it, for user, system and one-off. The repo went public at
stage 03, so every word of that is not merely stale but actively misleading: it
asks a reader to create a credential for something that needs none.

Delete it, renumber the steps that followed it, and while the flake-input
instructions are open, say what the input now installs: the default package
downloads a released binary, and `askance-source` builds from the tree for
anyone who wants that. A sentence on what an update means is worth having too —
the input still tracks the repository, and it is the manifest on `main` that
moves when a release lands.

Surgical, not a rewrite. Stage 06 rewrites the README adoption-first and moves
this material to a deployment guide; anything restructured here is restructured
twice.

## Acceptance criteria

- [ ] The "Give nix a GitHub token" section and the "Askance is a private
      repository" claim above it are gone, with no orphaned step numbers or
      cross-references left behind.
- [ ] The flake-input instructions name both packages and which one an import
      gets by default.
- [ ] Every remaining claim in the section matches the flake as task 01 and 02
      left it — attribute names, what `nix run` does, what the module installs.
- [ ] Links in the touched region still resolve.
