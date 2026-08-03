# Git workflow

## Branch naming

Pattern: `<feature>`

## Review process

### Finish sequence

Land the feature branch on the default branch, push it, then delete the
feature branch locally (the feature branch itself is never pushed):

    default=$(git symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null | sed 's@^origin/@@')
    default=${default:-main}
    feature=$(git branch --show-current)
    git switch "$default"
    git pull --ff-only          # land on top of whatever origin already has
    git merge --no-ff "$feature"
    git push
    git branch -d "$feature"

### Notes

- Default branch: `main`, on `origin` (github.com/tobico/askance, private).
- No PR is opened; work lands straight on the default branch and goes up on
  the next push. There is no review step between finishing a stage and it
  being on `origin/main` — the merge commit is the whole record.
- `--no-ff` is deliberate: each stage stays one merge commit in the history
  of `main`, so a stage can be read, or reverted, as a unit.
- If `git pull --ff-only` refuses because `main` has moved on `origin`,
  stop and sort it out by hand rather than merging blind.
- HTTPS over SSH: SSH to github.com fails on this machine with `Bad owner or
  permissions on …/ssh_config.d/20-systemd-ssh-proxy.conf`, so `origin` is an
  HTTPS URL authenticated by `gh`/`GITHUB_TOKEN`.
