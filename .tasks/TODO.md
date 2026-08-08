# Release pipeline

Pushing a `v*` tag produces a GitHub Release carrying four plain binaries —
`askance-linux-x64`, `askance-linux-arm64`, `askance-macos-x64`,
`askance-macos-arm64`, each with the viewer embedded — and a follow-up commit
on `main` updating the version-and-hash manifest that the binary flake (stage
04) reads. Assets are plain binaries rather than tarballs so the README's
install stays a single download-and-chmod (ADR-0004).

GitHub now hosts a runner for every architecture we ship, so **all four legs
build natively** — no cross-compilation anywhere. The Linux legs build static
musl binaries, which run on any distro and need no patchelf when the flake
fetches them.

Roadmap stage: [03: Release pipeline](docs/roadmaps/public-release/03-release-pipeline.md)

## Tasks

- [ ] 01: Shared viewer setup action — [details](01-viewer-setup-action.md)
- [ ] 02: Build matrix — [details](02-build-matrix.md)
- [ ] 03: Release on tag — [details](03-release-on-tag.md)
- [ ] 04: Manifest commit-back — [details](04-manifest-commit-back.md)
- [ ] 05: End-to-end proof — [details](05-end-to-end-proof.md)
