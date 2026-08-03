{
  description = "Askance — a service and CLI through which coding agents put questions to a human";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAllSystems (pkgs: {
        # Toolchain only. Packaging the server and CLI is a later stage.
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
            sqlite
            # The CLI derives `project`, `branch` and the Diff by shelling out
            # to git, so git is a runtime dependency and not just a habit.
            git
            # The web UI. cargo-leptos drives the two-target build; nixpkgs'
            # rustc already ships the wasm32 standard library, so there is no
            # rustup in the picture.
            cargo-leptos
            binaryen
            # The PWA icons are one SVG rasterized to the PNG sizes the manifest
            # and iOS need — see tools/generate-icons.sh.
            resvg
            # nixpkgs' rustc does not bundle rust-lld, and wasm32 links with
            # lld or not at all.
            lld
          ];

          env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt-tree);
    };
}
