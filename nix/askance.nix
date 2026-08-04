# The server, the CLI and the built site, from one build: the dependency tree
# here is leptos, syntect and sqlx, and compiling it twice to get two packages
# would cost more than the separation is worth.
{
  lib,
  rustPlatform,
  makeWrapper,
  git,
  # The wasm-side build tools, shared with the dev shell — see flake.nix.
  leptosTools,
}:

rustPlatform.buildRustPackage {
  pname = "askance";
  version = (lib.importTOML ../Cargo.toml).workspace.package.version;

  # Only what the build reads. `target/` and the development database sit in the
  # working tree beside these, and copying either into the store would make the
  # build depend on whatever the last `cargo leptos` left behind.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../crates
      ../assets
      ../style
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = leptosTools ++ [ makeWrapper ];

  # cargo-leptos takes no `--offline`, and the cargo runs it makes are the ones
  # that would reach for the network. Every dependency is vendored from the
  # lockfile, so say so once, for all of them.
  env.CARGO_NET_OFFLINE = "true";

  # cargo-leptos compiles the wasm half, runs wasm-bindgen over it, processes
  # the stylesheet and then builds the server binary. It knows nothing of the
  # CLI, which is a plain cargo build of its own.
  buildPhase = ''
    runHook preBuild

    cargo leptos build --release
    cargo build --release --package askance-cli

    runHook postBuild
  '';

  # The tests run the server and the CLI against each other over a socket, and
  # are the dev shell's `cargo test` to run. A build that repeated them would
  # buy nothing a checkout does not already have.
  doCheck = false;

  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin $out/share/askance
    cp -r target/site $out/share/askance/site
    install -m755 target/release/askance-server $out/bin/askance-server
    install -m755 target/release/askance $out/bin/askance

    # What `hash-files` wrote: which build each bundle under `site/pkg` came
    # from, which is the only way the server can name them. cargo-leptos leaves
    # it beside the binary it built, and it is kept beside the site it describes.
    install -m644 target/release/hash.txt $out/share/askance/hash.txt

    # The server takes its Leptos options from the environment only when
    # `LEPTOS_OUTPUT_NAME` is set, and otherwise falls back to a *relative*
    # `target/site` — so without this it would serve no wasm and no CSS from
    # anywhere but a working tree. The runtime configuration proper —
    # `ASKANCE_LISTEN`, `ASKANCE_DATABASE` — stays the caller's to set.
    #
    # `LEPTOS_HASH_FILES` is the other half of the workspace's `hash-files`: the
    # bundles in the site root are named by content, and a server that did not
    # know it would write the plain names, which nothing there answers to. The
    # hash file is named absolutely because Leptos looks for it beside the
    # binary, and that is a directory for binaries.
    wrapProgram $out/bin/askance-server \
      --set LEPTOS_OUTPUT_NAME askance \
      --set LEPTOS_SITE_ROOT $out/share/askance/site \
      --set LEPTOS_SITE_PKG_DIR pkg \
      --set LEPTOS_HASH_FILES true \
      --set LEPTOS_HASH_FILE_NAME $out/share/askance/hash.txt

    # The CLI shells out to git for the project, the branch and the Diff.
    wrapProgram $out/bin/askance \
      --prefix PATH : ${lib.makeBinPath [ git ]}

    runHook postInstall
  '';

  meta = {
    description = "A service and CLI through which coding agents put questions to a human";
    license = lib.licenses.mit;
    mainProgram = "askance-server";
    platforms = lib.platforms.unix;
  };
}
