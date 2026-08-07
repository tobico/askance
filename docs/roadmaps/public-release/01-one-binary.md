# 01. One binary

## Goal

`askance serve` is the server: one binary carries the CLI verbs and the
server both, `askance-server` no longer exists anywhere — cargo, nix, the
NixOS module, tests, docs — and `nix run` and the module behave exactly as
before.

## Decisions in force

- **Single-binary distribution is
  [ADR-0004](../../adr/0004-single-binary-distribution.md)** — read it first;
  the why (one download, no CLI/server version skew) and the accepted cost
  (~35 MB binary) live there.
- **The verb is `serve`**, chosen over `server` in the grilling session.
- The merge is expected to be small because the server is already a library:
  `askance_server::run(Config)` with `Config` a clap parser, and the CLI a
  clap parser in `askance-cli`. The `serve` verb hosts the server's flags as
  its own and starts the tokio runtime the CLI's other verbs don't need.
- The agent-facing contract must not change: `askance ask` / `askance guide`
  keep their exact behavior, stdout discipline included.

## Proposed tasks (provisional)

1. **`serve` verb** — `askance serve` runs the server with the existing
   flags. Accepts: UI and API serve on 8422; `askance ask` against it
   round-trips; server flags (`--listen`, database path, …) work under the
   verb.
2. **Retire `askance-server`** — the binary target goes, nix package installs
   one binary with `mainProgram = "askance"`, module execs `askance serve`.
   Accepts: `nix flake check` passes including the VM test; `nix run` still
   starts the server.
3. **Reference sweep** — tests, docs and comments that name `askance-server`
   updated (README gets a minimal touch only; stage 06 rewrites it).

## Re-verify at start

- `askance_server::run(Config)` is still the server's entry point and
  `Config` still derives `clap::Parser`.
- The VM test and module still reference `bin/askance-server` at the paths
  noted (`nix/askance.nix`, `nix/module.nix`).
- Whether anything else (scripts, docs/agents) invokes `askance-server` by
  name.
