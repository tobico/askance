# One binary: the server becomes `askance serve`

Going public, the install story has to be a download rather than a checkout,
and two binaries (`askance` and `askance-server`) meant tarballs, two files to
keep in sync, and a clumsier install command. The server was already a library
the CLI could call, so we fold it in: `askance serve` is the server, the
`askance-server` binary goes away, and a release asset is one plain
per-platform binary — which makes the documented install a single
download-and-chmod, and means an agent box carries exactly one artifact that
can never version-skew against itself.

The cost is that the CLI binary now carries the embedded viewer and the
syntect highlighting machinery (~35 MB where the CLI alone was small). For a
tool whose every host runs the server anyway, that is dead weight only in
principle; the CLI is invoked per-ask, not per-keystroke, and startup cost is
unaffected.

## The flake installs that binary rather than building one

The same reasoning reaches the flake, so `packages.default` fetches the
released binary and the source build stays reachable as `askance-source`. A
flake that compiled from source would hand the target adopter — a developer
installing a tool — a cold build of the whole Rust workspace plus a pnpm
viewer as their first experience of the project, which is the download story
above thrown away at the last step.

What the binary package reads is the manifest CI commits after each release: a
version, and a url and an SRI hash per nix system. Nothing in the flake is
hand-edited per release, because upkeep that costs anything goes undone, and a
stale hash is a broken install for the people least able to diagnose it.

The source package keeps its job. It is what `checks` build, so `nix flake
check` still proves the tree it is run against — including the NixOS VM test,
which pins `services.askance.package` to it deliberately. A test fed the
binary would be checking the last release rather than the branch under review,
which is the one thing a check must never do.

## Considered Options

- **Two binaries in a per-platform tarball** — no code change, but every
  install doc becomes download-untar-move-two-files, and the CLI and server
  can drift apart on a host that updates one and not the other.
