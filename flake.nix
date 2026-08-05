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

      # What building the two halves of the UI takes, wanted by the package and
      # the dev shell alike — so named once rather than left to drift apart.
      leptosTools =
        pkgs: with pkgs; [
          # cargo-leptos drives the two-target build; nixpkgs' rustc already
          # ships the wasm32 standard library, so there is no rustup in the
          # picture.
          cargo-leptos
          # wasm-opt, which cargo-leptos runs over the wasm in release mode.
          binaryen
          # nixpkgs' rustc does not bundle rust-lld, and wasm32 links with lld
          # or not at all.
          lld
        ];

      # What the SPA under `web/` is built and tested with. Named here for the
      # same reason as above: the dev shell and the check both take it, and a
      # pnpm in one that is not the pnpm in the other is a lockfile argument
      # waiting to happen.
      webTools =
        pkgs: with pkgs; [
          nodejs
          pnpm
        ];
    in
    {
      packages = forAllSystems (pkgs: rec {
        default = askance;
        askance = pkgs.callPackage ./nix/askance.nix { leptosTools = leptosTools pkgs; };
      });

      # The module runs the package above, so it closes over this flake rather
      # than looking for `pkgs.askance`, which is nowhere to be found.
      nixosModules = rec {
        default = askance;
        askance = import ./nix/module.nix self;
      };

      # `nix flake check` builds whatever is in here. The viewer's suite runs
      # anywhere node does; the VM test is offered only where a NixOS VM can be
      # booted at all, because it needs a Linux host to run the guest kernel on,
      # and on Darwin that check is simply absent rather than a failure.
      checks = forAllSystems (
        pkgs:
        {
          web = pkgs.callPackage ./nix/web-tests.nix { };
        }
        // nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          module = pkgs.callPackage ./nix/vm-test.nix { module = self.nixosModules.askance; };
        }
      );

      # `nix run` is the server, UI and all; the CLI is the other half of the
      # same derivation and has to be asked for by name.
      apps = forAllSystems (
        pkgs:
        let
          askance = self.packages.${pkgs.stdenv.hostPlatform.system}.askance;
        in
        {
          default = {
            type = "app";
            program = "${askance}/bin/askance-server";
            meta.description = "The Askance server, agent API and UI both";
          };
          askance = {
            type = "app";
            program = "${askance}/bin/askance";
            meta.description = "The Askance CLI, through which an agent asks";
          };
        }
      );

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages =
            (leptosTools pkgs)
            ++ (webTools pkgs)
            ++ (with pkgs; [
              cargo
              rustc
              clippy
              rustfmt
              rust-analyzer
              sqlite
              # The CLI derives `project`, `branch` and the Diff by shelling out
              # to git, so git is a runtime dependency and not just a habit.
              git
              # The PWA icons are one SVG rasterized to the PNG sizes the manifest
              # and iOS need — see tools/generate-icons.sh.
              resvg
            ]);

          env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt-tree);
    };
}
