# The viewer: the SolidJS SPA under `web/`, built by vite to the static files the
# server embeds (ADR-0003), and — with `runTests` on — the same sources put through
# typecheck and vitest instead.
#
# One file for both because they are one build: the same fileset, the same pinned
# node and pnpm, and the same store of dependencies, so a lockfile argument
# between `nix build` and `nix flake check` is not a thing that can happen.
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
  # Run the viewer's own suite rather than building it. The output is then a
  # stamp: a check is a thing that either builds or does not.
  runTests ? false,
}:

stdenvNoCC.mkDerivation (finalAttrs: {
  pname = if runTests then "askance-web-tests" else "askance-web";
  version = (lib.importTOML ../Cargo.toml).workspace.package.version;

  # `web/` and the assets vite copies verbatim into the site root, and nothing
  # else. The fixtures under `web/tests/fixtures` are the payloads `cargo test`
  # wrote, and the generated `web/src/api/types.ts` the same — both are committed,
  # so this reads them rather than needing the Rust half built first.
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
      # The service worker, the manifest and the icons — vite's `publicDir`.
      ../assets
    ];
  };

  nativeBuildInputs = [
    nodejs
    pnpm.configHook
  ];

  pnpmDeps = pnpm.fetchDeps {
    inherit (finalAttrs) version src;
    # Named for the build in both cases, so that turning `runTests` on does not ask
    # for a second copy of the same store under a second name.
    pname = "askance-web";
    sourceRoot = "${finalAttrs.src.name}/web";
    fetcherVersion = 2;
    hash = "sha256-yn9kKviolL5dXn+vLQ1hSHSgS4htUhBit/ZmfWLhdcs=";
  };

  sourceRoot = "${finalAttrs.src.name}/web";

  buildPhase = ''
    runHook preBuild
  ''
  + lib.optionalString (!runTests) ''
    pnpm build
  ''
  + lib.optionalString runTests ''
    # Typecheck as well as test: a component that only compiles because vite
    # erases the types would pass vitest and fail nobody, and the generated
    # `types.ts` is only worth generating if something checks the viewer against
    # it.
    pnpm typecheck
    pnpm test
  ''
  + ''
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
  ''
  + (
    if runTests then
      ''
        touch $out
      ''
    else
      ''
        cp -r dist $out
      ''
  )
  + ''
    runHook postInstall
  '';

  meta = {
    description = if runTests then "The Askance viewer's vitest suite" else "The Askance viewer, built";
    platforms = lib.platforms.unix;
  };
})
