# 04. Binary flake

## Goal

`nix run github:tobico/askance` fetches the released binary instead of
compiling the workspace, the NixOS module runs that same binary, and the
source build remains one attribute away — with the manifest that stage 03's
workflow maintains as the only thing that moves per release.

## Decisions in force

- **The binary package is the default** — the user chose this over keeping
  source as default, deliberately: the target adopter is a developer
  installing a tool, and a cold source build (full Rust workspace plus pnpm
  viewer) is the wrong first experience. The source package stays available
  by name, and stays what `checks` build, so the flake still proves the
  source tree.
- **The package reads the CI-committed manifest** (version, per-target url,
  sha256) rather than carrying hand-edited hashes — zero upkeep per release
  was the criterion that picked this design.
- Static musl linux binaries need no patchelf; the darwin binaries are used
  as-is. The binary package exists only for the four released systems.
- The module's `askance serve` invocation comes from stage 01; only its
  default package changes here.

## Proposed tasks (provisional)

1. **Manifest-fed package** — a `fetchurl`-based package per released
   system. Accepts: `nix run .#` on a fresh machine starts the server
   without compiling Rust; `nix run .#askance -- guide` prints the Guide
   (attribute names decided at stage start).
2. **Defaults flipped** — binary is `packages.default` and the module's
   default; source build reachable by name and still built by `nix flake
   check`. Accepts: VM test passes with whichever package the module now
   defaults to.
3. **Deployment docs touch-up** — the flake-input instructions lose the
   private-repo token section (obsolete once public; flagged by the audit).

## Re-verify at start

- Stage 03's manifest exists on main and a real release's hashes verify.
- The decision on which package the VM test exercises (binary needs network
  via fixed-output fetch; source keeps the test hermetic) — settle when the
  test is in front of you.
- `mainProgram` and app wiring after stage 01's rename.
