# The viewer's own test suite, run the way `nix flake check` runs everything
# else: offline, from a pinned toolchain, over exactly the sources that are
# committed.
#
# The pnpm store is fetched separately, as a fixed-output derivation named by
# `pnpmDeps.hash` — that is the one step allowed to reach the network, and the
# hash is what says the lockfile has not moved under us. Change `web/package.json`
# or `web/pnpm-lock.yaml` and this hash has to change with them; nix will print
# the one it wanted.
{
  lib,
  stdenvNoCC,
  nodejs,
  pnpm,
}:

stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "askance-web-tests";
  version = (lib.importTOML ../Cargo.toml).workspace.package.version;

  # `web/` and nothing else. The fixtures under `web/tests/fixtures` are the
  # payloads `cargo test` wrote, and the generated `web/src/api/types.ts` the
  # same — both are committed, so this check reads them rather than needing the
  # Rust half built first.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../web/index.html
      ../web/package.json
      ../web/pnpm-lock.yaml
      ../web/tsconfig.json
      ../web/vite.config.ts
      ../web/src
      ../web/tests
      # The stylesheet the app imports still lives up here until the cutover
      # moves it in — see `web/src/index.tsx`. Typechecking follows the import.
      ../style
    ];
  };

  nativeBuildInputs = [
    nodejs
    pnpm.configHook
  ];

  pnpmDeps = pnpm.fetchDeps {
    inherit (finalAttrs) pname version src;
    sourceRoot = "${finalAttrs.src.name}/web";
    fetcherVersion = 2;
    hash = "sha256-5pDu/Td3fpcLh23lh9cJdU21SluqJRqI+/40uCNi7iI=";
  };

  sourceRoot = "${finalAttrs.src.name}/web";

  # Typecheck as well as test: a component that only compiles because vite
  # erases the types would pass vitest and fail nobody, and the generated
  # `types.ts` is only worth generating if something checks the viewer against
  # it.
  doCheck = true;
  checkPhase = ''
    runHook preCheck

    pnpm typecheck
    pnpm test

    runHook postCheck
  '';

  # A check is a thing that either builds or does not; there is nothing here to
  # install, and a derivation with no output is not a thing nix will make.
  installPhase = ''
    runHook preInstall
    touch $out
    runHook postInstall
  '';

  meta = {
    description = "The Askance viewer's vitest suite";
    platforms = lib.platforms.unix;
  };
})
