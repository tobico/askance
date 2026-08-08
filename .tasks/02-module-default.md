# 02. The binary as the module's default

## What to build

A host that imports the flake's NixOS module gets the downloaded binary, not a
build. The module's `package` option defaults to the binary package and its
`defaultText` says so, so `nixos-rebuild` on a machine with no Rust toolchain
does not turn into a workspace compile.

The VM test goes the other way, deliberately: it pins
`services.askance.package` to `askance-source`. A test fed the binary would be
exercising whatever the last release contains rather than the tree it is run
against, which makes it worthless as a check on a branch — a `fetchurl` is a
fixed-output derivation, so the pin is about *what* is tested, not about
network access. Say that in a comment where the pin is, because the next reader
will otherwise see it as an oversight and helpfully remove it.

`checks` keep building the source package, which is the other half of the same
point.

## Acceptance criteria

- [ ] `services.askance.package` defaults to the binary package, with
      `defaultText` naming the attribute a reader would actually type.
- [ ] The VM test pins the package to `askance-source`, with a comment giving
      the reason above.
- [ ] `nix flake check` passes on linux, and the VM test it runs is exercising
      the source build.
- [ ] A NixOS configuration importing the module with nothing but
      `services.askance.enable = true` resolves to the binary package.
