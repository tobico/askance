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

## Considered Options

- **Two binaries in a per-platform tarball** — no code change, but every
  install doc becomes download-untar-move-two-files, and the CLI and server
  can drift apart on a host that updates one and not the other.
