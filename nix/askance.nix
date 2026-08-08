# One binary, viewer and all: `askance` carries the CLI verbs and the server both
# (ADR-0004), so there is one package to build here and nothing beside it to keep
# in sync.
{
  lib,
  rustPlatform,
  makeWrapper,
  git,
  # The viewer, built by vite — see web.nix. It is copied to where rust-embed
  # looks for it, and ends up inside the server binary rather than beside it.
  viewer,
}:

rustPlatform.buildRustPackage {
  pname = "askance";
  version = (lib.importTOML ../Cargo.toml).workspace.package.version;

  # Only what the build reads. `target/`, `web/node_modules` and the development
  # database sit in the working tree beside these, and copying any of them into
  # the store would make the build depend on whatever the last one left behind.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../crates
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ makeWrapper ];

  # Where `#[folder]` points — see `crates/server/src/viewer.rs`. Writable because
  # nothing here reads it after cargo has, but the store copy is not, and cargo
  # would be reading through a read-only directory to no purpose.
  preBuild = ''
    mkdir -p web
    cp -r ${viewer} web/dist
    chmod -R u+w web/dist
  '';

  # The binary's package by name rather than the whole workspace:
  # `askance-render`'s own default features turn on the TypeScript emitter, which
  # is a test's business, and a workspace-wide build would unify it into the
  # release binary.
  cargoBuildFlags = [
    "--package"
    "askance-cli"
  ];

  # The tests run the server and the CLI against each other over a socket, and
  # are the dev shell's `cargo test` to run. A build that repeated them would
  # buy nothing a checkout does not already have.
  doCheck = false;

  # The CLI shells out to git for the project, the branch and the Diff. The server
  # needs nothing set at all: the viewer is inside it, and where its database and
  # socket are — `ASKANCE_DATABASE`, `ASKANCE_LISTEN` — stays the caller's to say.
  postInstall = ''
    wrapProgram $out/bin/askance \
      --prefix PATH : ${lib.makeBinPath [ git ]}
  '';

  meta = {
    description = "A service and CLI through which coding agents put questions to a human";
    license = lib.licenses.mit;
    mainProgram = "askance";
    platforms = lib.platforms.unix;
  };
}
