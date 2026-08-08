# 03. Nix job

## What to build

`nix flake check` in CI, on **pushes to `main` only** — not on pull requests.
It is the slow, hermetic path, and holding every branch's feedback behind it
would cost more than it buys; landing on `main` is where it earns its place.

It is also the only automated check the NixOS module has. `flake.nix` exposes
two checks: the viewer's suite built hermetically, and — on Linux hosts only —
a VM test that boots the module. Stages 03 and 04 of the roadmap rework the
release pipeline and the flake, so this is the guard those stages will lean on.

Two things to settle while building it, neither of which is obvious from here:

- **Whether the runner can boot the VM test at all.** It needs KVM on the
  host. If the runner image cannot provide it, say so plainly and decide with
  the user whether the nix job runs the remaining checks without it or is
  dropped — do not quietly narrow `nix flake check` to something weaker and
  report success.
- **Whether a nix store cache is worth it.** A cold store rebuilds rust from
  scratch every run. Measure the uncached run first, then decide.

Expressing this as a conditional job in the existing workflow or as a second
workflow file are both fine; pick one and say why in a comment.

## Acceptance criteria

- [ ] `nix flake check` runs and reports **green** on a push to `main`
- [ ] It does **not** run on pull requests
- [ ] The VM test is confirmed either to run or to be genuinely unavailable on
      the runner, with the outcome recorded
- [ ] The run's wall-clock time is known and noted, so a later stage can judge
      whether caching is worth adding
