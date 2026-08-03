# 01. The flake package

## What to build

`nix run` on a fresh clone starts the server, UI and all, and `nix run .#askance`
runs the CLI. One derivation builds both binaries and the site directory — the
dependency tree here is leptos, syntect and sqlx, and compiling it twice to get
two packages is not worth the separation.

`cargo leptos` drives the build, as it does in the dev shell, so the sandbox needs
what the wasm half needs: `cargo-leptos` itself, `lld` (nixpkgs' rustc has no
rust-lld, and wasm32 links with lld or not at all), and `binaryen` for `wasm-opt`
in release mode. The `wasm-bindgen` pin in the workspace has to keep matching the
one built into the nixpkgs `cargo-leptos` — the same constraint the dev shell
already lives under, now also a build-time one. Vendor the dependencies from the
committed lockfile so no new flake input is needed; the input set stays at
nixpkgs alone.

Then wrap the binaries, because neither works from an arbitrary working directory
otherwise:

- the server with `LEPTOS_OUTPUT_NAME` and `LEPTOS_SITE_ROOT` pointing at the
  site directory installed beside it, which is the only way the packaged binary
  takes its options from the environment rather than falling back to a relative
  `target/site`;
- the CLI with `git` on its `PATH`, which it shells out to for the project, the
  branch and the Diff.

Neither wrapper is a substitute for the runtime configuration: `ASKANCE_LISTEN`,
`ASKANCE_DATABASE` and `ASKANCE_SERVER` stay the caller's to set, and the module
in task 02 is what sets them.

Leave the dev shell as it is. Both it and the package want the same tools, so
express that once rather than letting the two drift.

## Acceptance criteria

- [ ] `nix run` in a clean clone of the current commit serves the pending list,
      with the wasm and the stylesheet under `/pkg/` and the manifest, icons and
      service worker in the site root
- [ ] The server binary run from a directory with no `target/` finds its site
      files — the wrapper, not the caller, supplies the site root
- [ ] `nix run .#askance -- --help` works, and an `askance ask` from a git
      repository captures project, branch and Diff with no `git` in the ambient
      `PATH`
- [ ] `nix build` succeeds with the network off after the inputs are fetched, and
      `nix flake check` passes
- [ ] The dev shell still provides everything the README's Development section
      claims it does
