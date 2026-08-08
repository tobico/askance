# The released binary, fetched rather than built: an adopter running
# `nix run github:tobico/askance` should get the tool, not a cold compile of the
# Rust workspace and the pnpm viewer beside it. Which version to fetch, from
# where, and what it must hash to all come out of `nix/release.json`, which
# `release.yml` commits to `main` after every tag — so nothing in here is edited
# per release.
#
# The build from this tree is `askance-source`, one attribute away.
{
  lib,
  stdenvNoCC,
  fetchurl,
  makeWrapper,
  git,
}:

let
  release = lib.importJSON ./release.json;
  inherit (stdenvNoCC.hostPlatform) system;

  # Keyed by nix system name, which is what the manifest records and what is in
  # hand here. A Release carries four assets and no others, and there is nothing
  # to fall back to — so an unlisted system fails at evaluation, naming the
  # package that does build anywhere, rather than fetching a url that 404s.
  asset =
    release.systems.${system}
      or (throw "askance: the release manifest has no binary for ${system} — build askance-source instead");
in

stdenvNoCC.mkDerivation {
  pname = "askance";

  # The manifest's version rather than `Cargo.toml`'s. The two answer different
  # questions: this package is whatever the last release published, and the tree
  # it sits in has usually moved on past it.
  version = release.version;

  src = fetchurl { inherit (asset) url hash; };

  # One binary, not an archive.
  dontUnpack = true;

  # The linux assets are static musl and the darwin ones are used as they were
  # built, so there is no interpreter to patch. Nor anything to strip: what
  # ships is the file the release workflow built and ran, byte for byte, and the
  # hash above is a promise about that file.
  dontStrip = true;

  nativeBuildInputs = [ makeWrapper ];

  # `install` supplies the executable bit the asset does not carry — a GitHub
  # Release serves a plain file, and the mode did not survive the upload.
  installPhase = ''
    runHook preInstall
    install -D -m755 $src $out/bin/askance
    runHook postInstall
  '';

  # The CLI shells out to git for the project, the branch and the Diff, exactly
  # as it does when built from source — so it gets the same wrapper here, since
  # a downloaded binary gets none of what buildRustPackage arranges for free.
  postInstall = ''
    wrapProgram $out/bin/askance \
      --prefix PATH : ${lib.makeBinPath [ git ]}
  '';

  meta = {
    description = "A service and CLI through which coding agents put questions to a human";
    license = lib.licenses.mit;
    mainProgram = "askance";
    # Only where a Release actually has an asset, read off the manifest so this
    # follows the workflow's build matrix rather than restating it.
    platforms = lib.attrNames release.systems;
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
  };
}
