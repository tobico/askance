# Git workflow

## Branch naming

Pattern: `<feature>`

## Review process

### Finish sequence

**Unstacked branch** — the normal case:

1. Push the current feature branch to origin.
2. Open a **draft** PR (`gh pr create --draft`). Title = feature name;
   body = summary of the completed tasks (name the stage and roadmap if
   this was a roadmap stage). Return the PR URL.

**Stacked branch** — created with `gh stack init`/`gh stack add`:

1. `gh stack submit --auto` — pushes every branch in the stack, opens a
   PR for each branch that lacks one, repoints the base of existing PRs,
   and creates or updates the Stack on GitHub. **Always pass `--auto`**:
   bare `gh stack submit` opens an interactive editor an agent can't
   drive. With `--auto` new PRs are created as drafts (`--open` would
   make them ready for review — don't).
2. `--auto` auto-generates PR titles, so correct this branch's PR:

       gh pr edit <branch> --title '<feature name>' --body '<summary>'

   Leave the stack's other PRs alone — they belong to finished stages.
3. `gh stack view` lists the stack's PRs. Return this branch's PR URL.

### Stacking roadmap stages

When a new roadmap stage's predecessor is finished but its PR is **not
yet merged**, and the new stage builds directly on that work, put the new
stage on a stacked branch. Only branch off `main`, unstacked, when the
stage is genuinely independent of any unmerged predecessor.

Prerequisite: `gh extension install github/gh-stack` (skip if
`gh stack --help` already works).

- **First stacked stage** — adopt the predecessor as the stack bottom:

      gh stack init <predecessor-branch> <new-branch>

  `init` adopts branches that already exist and creates the ones that
  don't, so the predecessor's history is left untouched.

- **Stack already exists** — extend it:

      gh stack checkout <any-branch-in-the-stack>
      gh stack top
      gh stack add <new-branch>

Both keep the branch-naming pattern. Leave the branch empty at creation
time; the plan commit lands on it normally (`gh stack add` only commits
when passed `-m`/`-A`/`-u`).

### Updating a stack after review

Don't rebase stacked branches by hand. From any branch in the stack:

- `gh stack sync` — fetches, cascade-rebases each branch onto its updated
  parent, force-pushes atomically, and re-links the stack on GitHub. Use
  it after `main` moves or after an earlier PR in the stack merges. It
  never opens PRs.
- `gh stack rebase` — resolve conflicts interactively when `sync` reports
  one and backs out.
- Re-run `gh stack submit --auto` after adding a new branch to an
  existing stack.

### Exception: the release manifest

`.github/workflows/release.yml` pushes one commit straight to `main` with no
pull request — `chore: release manifest for <tag>`, touching
`nix/release.json` and nothing else. It is the only write to `main` that
bypasses the process above, and it is deliberate.

That manifest is the entire interface to the binary flake: a version, plus a
url and an SRI hash per nix system, which the flake fetches by and verifies
against with nothing hand-edited in between. Upkeep has to be zero per
release. A manifest that needed a merge would go stale the first time
somebody was busy, and a stale one is a broken install for everyone
downstream — which is exactly the audience least able to diagnose it.

What follows from the exception:

- The commit is authored by `github-actions[bot]`, so `main`'s history
  carries commits belonging to no person. That is the intent: nobody wrote
  it, and attributing it to whoever pushed the tag would be a fiction.
- A push made with the workflow's own `GITHUB_TOKEN` starts no workflow run,
  so CI never sees this commit. That is what stops the push from looping —
  and it is only safe because the commit touches one generated file that no
  check reads. Anything else landing this way would be unreviewed *and*
  unchecked.
- Branch protection on `main` requiring a pull request would reject the push
  and fail the release run. If protection is ever turned on, the workflow's
  identity needs a bypass, or this job needs another route.

### Notes

- Default branch: `main`, on `origin` (github.com/tobico/askance, **public**
  since 2026-08-08 — flipped at the start of roadmap stage 03, which cannot be
  exercised otherwise: release assets on a private repo need auth to download).
- PRs open as draft. Change "draft" to "ready" here to open ready-for-review.
- This replaced a direct-merge process on 2026-08-08, at the start of the
  `ci` stage — commits up to `e35ea47` landed on `main` without a PR, so
  the history before that point shows merge commits and no PR references.
- HTTPS over SSH: SSH to github.com fails on this machine with `Bad owner or
  permissions on …/ssh_config.d/20-systemd-ssh-proxy.conf`, so `origin` is an
  HTTPS URL authenticated by `gh`/`GITHUB_TOKEN`. That token needs the
  `workflow` scope to push anything under `.github/workflows/`.
