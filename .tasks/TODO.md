# Binary flake

`nix run github:tobico/askance` fetches the binary a GitHub Release already
built, instead of compiling the Rust workspace and the pnpm viewer on the
adopter's machine. The package reads `nix/release.json` — the version, url and
SRI hash per nix system that the release workflow commits after every tag — so
nothing in the flake is hand-edited per release. The NixOS module runs that
same binary, and the source build stays one attribute away as `askance-source`,
still what `nix flake check` proves.

Stage 03 deleted its rehearsal Release and reverted the manifest, so `main`
carries neither today. Task 01 opens by cutting `v0.1.0-rc.1` — a pre-release,
kept until v0.1.0 supersedes it at stage 06 — because the package cannot be
written against a manifest that does not exist.

Roadmap stage: [04: Binary flake](docs/roadmaps/public-release/04-binary-flake.md)

## Tasks

- [x] 01: Manifest-fed binary package — [details](01-manifest-fed-package.md)
- [x] 02: The binary as the module's default — [details](02-module-default.md)
- [ ] 03: README loses the private-repo token — [details](03-readme-token.md)
