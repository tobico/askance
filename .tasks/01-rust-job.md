# 01. Rust job

## What to build

The project's first GitHub Actions workflow, carrying the Rust half of the
checks. It triggers on **pushes to `main` and on pull requests** — a branch with
an open PR would otherwise run twice for the same commit.

The job checks out, installs a **pinned Rust 1.91** toolchain with `clippy` and
`rustfmt`, caches the cargo build, and runs four checks. All four are green on
`main` as it stands, so this lands green:

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets` with warnings denied
- `cargo fmt --all --check`
- a **drift guard on the generated TypeScript**: `cargo test` writes
  `web/src/api/types.ts` through ts-rs (see `.cargo/config.toml`), so a working
  tree that is no longer clean after the test run means the committed wire types
  have drifted from the Rust ones. Fail the job, and say what drifted.

The viewer does **not** need building first — `web/dist` is gitignored and the
embed is declared `allow_missing`, so the Rust crates compile without it.

Pin action versions and the runner image explicitly rather than floating on
`@latest` or `ubuntu-latest`; check what is current when you write it. The
toolchain of record lives in the flake (nixos-25.11: rust 1.91.1) and CI should
not drift far from it.

Finish by pushing the branch and opening the **draft PR** — the workflow cannot
run on this branch until one exists, and the remaining tasks need somewhere to
watch it. The `GITHUB_TOKEN` in use has the `workflow` scope, which pushing
anything under `.github/workflows/` requires.

## Acceptance criteria

- [ ] The workflow runs on the PR and the Rust job reports **green**
- [ ] Triggers are pushes to `main` and pull requests — one run per commit, not two
- [ ] Deliberately breaking a test, a clippy lint, or formatting turns the job
      red locally before it is reverted
- [ ] Editing a `#[ts(export)]` type without committing the regenerated
      `types.ts` is caught by the drift guard
- [ ] Rust is pinned to 1.91, not tracking `stable`
