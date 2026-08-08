# 01. Manifest-fed binary package

## What to build

A nix package that downloads the released `askance` binary for the host system
and installs it, rather than compiling anything — and the real release it needs
in order to exist at all.

Two halves, in this order, because the second cannot be tested without the
first:

**A release to fetch.** Push the tag `v0.1.0-rc.1` from `main`. The release
workflow builds the four assets, publishes them under the tag, and commits
`nix/release.json` back to `main`. The hyphen makes it a pre-release, so
`releases/latest` stays clear for v0.1.0 at stage 06 — this manifest is kept,
not reverted the way stage 03's rehearsal was. Bring the manifest commit onto
this branch before writing anything that reads it.

**A package that reads the manifest.** Keyed by nix system name, which is what
the manifest records and what the flake has in hand. The linux assets are
static musl and the darwin ones are used as-is, so no patchelf and no
interpreter fixing — but two things the source package gets for free have to be
done by hand here: the asset carries no executable bit (it survives a GitHub
Release as a plain file), and the CLI shells out to git for the project, branch
and Diff, so git belongs on `PATH` through a wrapper exactly as
`nix/askance.nix` already does it.

Rename in the same move, per the naming decided at stage start: `askance` is
the binary and `askance-source` is the build from the tree. `packages.default`
follows `askance` and so flips with no separate change, and both `apps` — the
`serve` wrapper and the bare CLI — come from the binary.

The version in the manifest is the authority for the package's `version`;
don't read `Cargo.toml` for it, since the two answer different questions.

## Acceptance criteria

- [ ] `v0.1.0-rc.1` exists as a GitHub **pre-release** carrying all four
      assets under their documented names.
- [ ] `nix/release.json` is on `main`, names version `0.1.0-rc.1` and all four
      nix systems, and each recorded hash matches `nix hash file --sri` of the
      asset downloaded from the url beside it.
- [ ] `nix build .#askance` completes without compiling the Rust workspace or
      running pnpm.
- [ ] `nix run .#` serves the viewer — the document it returns names a hashed
      bundle, not the 503 an unembedded build gives.
- [ ] `nix run .#askance -- guide` prints the Guide, and the CLI finds git
      without git being on the caller's `PATH`.
- [ ] `nix build .#askance-source` still builds from the tree.
