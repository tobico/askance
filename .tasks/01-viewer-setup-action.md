# 01. Shared viewer setup action

## What to build

The release workflow needs the same pinned pnpm, pinned Node and
`--frozen-lockfile` install that `ci.yml`'s Viewer job already performs. Rather
than carry those pins in two workflows — exactly the quiet drift the CI
comments warn against — extract them into a composite action that both call.

The action does the setup and nothing beyond it: install pnpm, install Node
with the pnpm store cached against `web/pnpm-lock.yaml`, and install the
viewer's dependencies. What happens next differs by caller — CI typechecks and
tests, the release workflow runs `pnpm build` — so neither belongs in the
action.

This is a pure refactor and lands before anything depends on it, so CI going
green with no behaviour change is the whole proof.

## Acceptance criteria

- [ ] The pinned pnpm version, Node version and action versions appear in
      exactly one place in the repository
- [ ] `ci.yml`'s Viewer job calls the action in place of its own setup steps,
      and still runs typecheck and test as separate steps
- [ ] The Viewer job passes on this branch's PR, doing the same work it did
      before — a `package.json` that has drifted from the lockfile still fails
      the install
- [ ] The action carries comments explaining the pins, in keeping with the
      surrounding workflow's style
