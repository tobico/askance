# Git workflow

## Branch naming

Pattern: `<feature>`

## Review process

### Finish sequence

Land the feature branch on the default branch, then delete it locally
(never push the feature branch in this mode):

    default=$(git symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null | sed 's@^origin/@@')
    default=${default:-main}
    feature=$(git branch --show-current)
    git switch "$default"
    git merge --no-ff "$feature"
    git push               # only if a remote exists
    git branch -d "$feature"

### Notes

- Default branch: main
- No PR is opened; work lands straight on the default branch.
- This repository has no remote, so the `git push` step is a no-op until one
  is added.
