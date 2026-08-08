# 05. End-to-end proof

## What to build

Everything before this was proved a piece at a time. This task drives the whole
pipeline once, for real, with a throwaway pre-release tag — because the parts of
a release workflow that break are the parts no dry run exercises: the token's
scopes, the push to a protected branch, an asset URL that only resolves once the
Release is actually public.

Push the tag, watch the run, and fold what it teaches back into the workflow.
The verification that matters is the adopter's: download a binary the way the
README will tell someone to, on a machine that never built it, and run it.

Then clean up. The throwaway Release, its tag, and any manifest commit it left
on `main` are scaffolding, and leaving them behind would mean the project's
first public release is not `v0.1.0`.

This is the last task of the stage. `v0.1.0` itself is not tagged here — that is
the go-live step, after the flake and the docs land.

## Acceptance criteria

- [ ] A throwaway pre-release tag drives the pipeline end to end: four assets
      published, manifest committed to `main`
- [ ] A binary downloaded from its public release URL onto a machine that did
      not build it runs `askance --help` and serves the viewer
- [ ] Every hash in the committed manifest verifies against its published asset
- [ ] Anything the real run taught is folded back into the workflow, and the
      workflow is re-run if the fix was load-bearing
- [ ] The throwaway Release, its tag, and its manifest commit are gone; `main`
      is left as though the rehearsal never happened
- [ ] No `v0.1.0` tag is pushed
