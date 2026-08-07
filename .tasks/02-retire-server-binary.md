# 02. Retire the `askance-server` binary

## What to build

The `askance-server` binary target and its `main.rs` go, and everything that
reached the server through them reaches it through `askance serve` instead. The
crate stays — it is the library the CLI now calls and the one the server's own
tests exercise; it just stops producing a second binary.

Three things point at `bin/askance-server` today and all three move:

- The nix package builds one cargo package rather than two, and its
  `mainProgram` becomes `askance`. The package installs one binary, and the
  wrapper that puts git on the CLI's `PATH` now wraps the only binary there is.
- The NixOS module's `ExecStart` execs `askance serve` with the same
  `--listen` and `--database` arguments it passes today.
- The flake's `apps.default` — what `nix run` runs — points at `bin/askance`
  and passes the `serve` verb.

The VM test doesn't name the binary itself; it exercises the module, so it is
the check that the module rewiring actually starts a server. `nix flake check`
is the acceptance signal.

## Acceptance criteria

- [ ] `crates/server/src/main.rs` and the crate's `[[bin]]` section are gone,
      and the library crate still builds and its tests still pass
- [ ] The nix package installs exactly one binary, named `askance`, with
      `mainProgram = "askance"`
- [ ] `nix flake check` passes, VM test included — the service comes up, the
      database lands in the state directory, and a Set round-trips
- [ ] `nix run` still starts the server, UI and API both
- [ ] `cargo build` produces one binary from the workspace's shipped packages
