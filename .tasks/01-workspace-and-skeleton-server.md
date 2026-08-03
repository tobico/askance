# 01. Workspace, dev shell, and skeleton server

## What to build

The ground the rest of the stage stands on: a buildable repo and a server
process that runs. Cargo workspace with three crates — `schema` (shared
Question Set / Response types, later compiled to WASM), `server` (axum
binary), `cli` (binary, fleshed out in task 04) — plus a minimal `flake.nix`
devShell providing the Rust toolchain (this machine has no cargo outside
Nix; full packaging is stage 05's job, the devShell here is toolchain only).

The server binary opens (creating if absent) a SQLite database at a
configurable path, binds a configurable address/port suitable for
localhost/tailnet use (no app-level auth — tailnet is the perimeter), and
answers a health-check route. Use axum 0.8's `{param}` route syntax
throughout; REST routes will live under `/api/v1/`.

## Acceptance criteria

- [ ] On a fresh checkout, `nix develop` provides the toolchain and
      `cargo build` succeeds for the whole workspace
- [ ] Running the server creates the SQLite file when missing and a `curl`
      to the health-check route succeeds
- [ ] Database path and bind address/port are configurable (flags or env)
      with sensible localhost defaults
- [ ] The `schema` crate depends on no server-only crates (no tokio, axum,
      or SQLite deps) — it must stay compilable for `wasm32-unknown-unknown`
      in stage 02
