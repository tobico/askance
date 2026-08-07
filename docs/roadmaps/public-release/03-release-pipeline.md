# 03. Release pipeline

## Goal

Pushing a `v*` tag produces a GitHub Release carrying four plain binaries —
`askance-linux-x64`, `askance-linux-arm64`, `askance-macos-x64`,
`askance-macos-arm64`, each with the viewer embedded — and a follow-up commit
on main updating a version-and-sha256 manifest that stage 04's flake package
reads.

**Prerequisite: the repo is public** (decided in the roadmap quiz; the
history audit of 2026-08-07 cleared it). Private-repo assets need auth to
download, so nothing downstream can be exercised until the flip.

## Decisions in force

- **Tag-driven** (chosen over manual dispatch and per-commit releases);
  first public version is **v0.1.0**, matching Cargo.toml.
- **Assets are plain binaries, not tarballs** — a consequence of
  [ADR-0004](../../adr/0004-single-binary-distribution.md): one file makes
  the README's install a single download-and-chmod. Friendly names
  (`linux-x64`, not target triples) because they appear verbatim in that
  command.
- **Linux builds are static musl**, so one binary runs on any distro — the
  whole point of offering a binary. macOS x64 builds on the Intel runner,
  arm64 on the Apple-silicon runner. Linux arm64 was explicitly added
  (GitHub's arm runners are free for public repos).
- **No macOS signing or notarization** — curl-downloaded binaries carry no
  quarantine attribute, and notarization needs a paid account the project
  doesn't have.
- **CI commits the manifest to main after publishing** (chosen over manual
  hash bumps and a separate bin-flake repo): upkeep must be zero per release
  or the flake goes stale. The workflow therefore needs contents write
  permission; shape and path of the manifest are stage 04's interface —
  agree them here, e.g. version plus per-target url and sha256.
- The viewer is built with pnpm in CI and embedded exactly as the nix build
  does today.

## Proposed tasks (provisional)

1. **Build matrix** — the four targets produce runnable binaries with the
   viewer embedded. Accepts: artifact from each leg runs `askance --help`
   and serves the UI; linux binaries pass `ldd` as static.
2. **Release on tag** — a pushed pre-release tag creates a GitHub Release
   with the four named assets. Accepts: `curl -L .../releases/latest/download/askance-linux-x64`
   yields a working binary.
3. **Manifest commit-back** — after assets publish, main gains the updated
   manifest. Accepts: hashes verify against the downloaded assets.
4. **End-to-end proof** — a throwaway pre-release tag drives the whole
   pipeline once; findings folded back before v0.1.0.

## Re-verify at start

- Stage 01 landed (single binary; asset-per-platform assumption holds).
- The repo is public; if not, flip it first.
- Current GitHub runner labels for macOS Intel and linux arm — these shift
  under projects; pick what exists then.
- Whether sqlx/sqlite and the rest of the dependency tree build cleanly for
  `*-unknown-linux-musl` with the pinned toolchain.
