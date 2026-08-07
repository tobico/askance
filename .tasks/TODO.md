# One binary

`askance serve` becomes the server. One binary carries the CLI verbs and the
server both, the `askance-server` binary target goes away, and `nix run`, the
NixOS module and the agent-facing CLI behave exactly as they did before.

The why is [ADR-0004](../docs/adr/0004-single-binary-distribution.md): going
public, the install story has to be a download rather than a checkout, and two
binaries meant tarballs, two files to keep in sync, and a CLI that could
version-skew against the server on the same host. The merge is small because
the server is already a library — `askance_server::run(Config)` with `Config` a
clap parser — so the `serve` verb hosts the server's flags as its own and
starts the tokio runtime the other verbs don't need.

The library crate keeps the name `askance-server`; only its `[[bin]]` target
goes. The `tracing_subscriber` setup that binary carried moves into the CLI,
which takes its own `tracing-subscriber` dependency for it.

Roadmap stage: [01: One binary](docs/roadmaps/public-release/01-one-binary.md)

## Tasks

- [x] 01: `askance serve` — [details](01-serve-verb.md)
- [ ] 02: Retire the `askance-server` binary — [details](02-retire-server-binary.md)
- [ ] 03: Reference sweep — [details](03-reference-sweep.md)
