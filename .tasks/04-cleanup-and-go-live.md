# 04. Cleanup and go-live prep

## What to build

The last of the staleness, and proof that the install story in the new README
actually works — rehearsed against the release that already exists rather than
taken on trust.

**Verify the install path for real.** `v0.1.0-rc.1` is published and
`nix/release.json` points at it, so the README's instructions can be exercised
today:

- The curl one-liner. Note that it is written against
  `releases/latest/download`, and `v0.1.0-rc.1` is marked **pre-release** — so
  `latest` may not resolve to it. Whether the one-liner works before v0.1.0
  ships is itself worth knowing, and if it cannot be proven now, say so in the
  checklist rather than claiming it works.
- `nix run github:tobico/askance`, which goes through the binary flake and the
  manifest.
- A server running an older version showing the Update Notice, with its link
  now resolving to the README section written in task 03.

**Staleness.** `docs/PLAN.md` predates most of what shipped — read it and either
bring it in line or note plainly what it now is. The audit-flagged README token
section is **already gone** (commit `75efa18`), so there is nothing to do there.

**Explicitly not done:** the machine-specific SSH note in
`docs/agents/git-workflow.md` stays. The brief called it stale, but it is an
agent-facing doc rather than a shop window, and the breakage it describes is
live — pushing PR #7 hit exactly it. Leave it alone.

**Go-live is not tagged here.** `release.yml` fires on a `v*` tag and pushes
`nix/release.json` to `main`, so v0.1.0 has to be tagged on a commit already on
`main` — after this stage's PR and PR #7 both merge. This task ends with a
checklist recording what to run then, and the human tags.

## Acceptance criteria

- [ ] Each install path in the README is actually attempted, and the result
      recorded — including an honest note where `releases/latest/download`
      cannot be proven while the only release is a pre-release
- [ ] The Update Notice's link is followed end to end and lands on a real
      section
- [ ] `docs/PLAN.md` is either updated or carries a clear note about what it now
      describes
- [ ] The SSH note in `docs/agents/git-workflow.md` is untouched
- [ ] A go-live checklist exists saying what to tag, on what, and what to check
      afterwards — written so it can be followed without this context
- [ ] No `v0.1.0` tag is created by this task
