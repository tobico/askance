# Nix packaging

`nix run` builds and starts Askance from a fresh checkout, and a NixOS module
runs it as a systemd service on the box the agents live on — surviving reboots,
with the CLI on `PATH`. Until now the flake has carried a dev shell and nothing
else: the server has only ever run out of a working tree, from a `target/site`
that `cargo leptos` last wrote.

The catch the packaging has to solve is that site root. The server reads its
Leptos options from the environment only when `LEPTOS_OUTPUT_NAME` is set and
otherwise falls back to a *relative* `target/site`, so a store-path binary run
from anywhere else finds no wasm and no CSS. The package wraps both binaries
rather than leaving that to whoever runs them: the server pointed at the site
directory it was built with, the CLI at the `git` it shells out to.

The service stays on loopback and behind `tailscale serve`, which the README
already documents — the module's job is the systemd unit, a dedicated user and a
state directory, not a second copy of the HTTPS story.

Roadmap stage: [05: Nix packaging + deployment](docs/roadmaps/v1/05-nix-packaging.md)

## Tasks

- [x] 01: The flake package — [details](01-flake-package.md)
- [x] 02: The NixOS module — [details](02-nixos-module.md)
- [ ] 03: A VM test of the module — [details](03-vm-test.md)
- [ ] 04: Deploy it on the real box — [details](04-deploy.md)
