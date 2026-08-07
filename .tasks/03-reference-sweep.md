# 03. Reference sweep

## What to build

The remaining mentions of `askance-server` **as a binary** — in docs and in
comments — brought in line with the one binary that now exists. References to
`askance_server::` as a library are correct and stay.

What is known to be left after tasks 01 and 02:

- The README's development instructions run the server as
  `cargo run -p askance-server`, in the quick-start and again in the note about
  the viewer being served off disk in a debug build. Both become the
  one-binary invocation. **Keep this minimal** — stage 06 of the roadmap
  rewrites the README, so this is a correctness touch, not a rewrite.
- Comments that describe the package as producing two binaries: the header and
  `cargoBuildFlags` note in the nix package, the module's note about "both
  binaries" landing on `PATH` and about reading `askance-server --help`, and
  the `rust-embed` comment in the server crate's manifest that names
  `cargo run -p askance-server`.

Finish by grepping the tree for `askance-server` and confirming every survivor
is the library crate being named as a cargo dependency or a Rust path.

## Acceptance criteria

- [ ] The README's development commands start the server as the one binary
      does, and nothing in it tells a reader to build `askance-server`
- [ ] Comments in the nix package, the NixOS module and the server crate's
      manifest describe one binary
- [ ] `grep -rn askance-server` over the tree (excluding `target/`) returns
      only cargo dependency declarations and `askance_server::` library paths
- [ ] `cargo test` and `nix flake check` still pass
