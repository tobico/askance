# 05. Nix packaging + deployment

## Goal

`nix run` builds and starts askance from a fresh checkout; the NixOS module
runs it as a systemd service on the box the agents live on, surviving
reboots, with the CLI on `PATH`.

## Decisions in force

- **Nix flake providing package + NixOS module** (grilling session Q12) —
  chosen over a plain systemd unit to avoid drift; the host is NixOS.
- **Module options**: port and SQLite db path at minimum; service runs as a
  dedicated user with a state directory.
- **Same box as the agents** (grilling session Q12a) — CLI's localhost
  default just works; other tailnet machines reach it via the env var
  override.
- `tailscale serve` configuration stays outside the module in v1 (it's
  host-level tailscale config), but the module's docs say what to run.

## Proposed tasks (provisional)

1. **Flake package** — build server + CLI (likely via crane or
   rustPlatform); `nix run .#askance-server` works.
   - Fresh clone builds with flakes enabled, no impurities
2. **NixOS module** — systemd service, state dir, port/db-path options,
   packages the CLI into the environment.
   - Module evals in a NixOS VM test or the real host config; service
     restarts cleanly with state intact
3. **Deploy + smoke test** — enable on this box, document the rebuild step
   and the `tailscale serve` line.
   - After reboot the service is up and a CLI ask round-trips

## Re-verify at start

- Reorderable: only stage 01 is required; if run before 04, drop the
  `tailscale serve` doc bits to whatever exists yet.
- Leptos build outputs (SSR + hydration wasm) and how they package under nix
  — check current cargo-leptos nix patterns at implementation time.
- Whether the repo picked cargo-leptos or plain trunk/cargo in stage 02; the
  packaging approach follows it.
