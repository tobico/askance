# 01. `askance serve`

## What to build

The CLI grows a `serve` verb that runs the server. It hosts the server's
existing flags as its own — the same long names, env vars and defaults the
`askance-server` binary parsed — and starts the tokio runtime that the CLI's
other verbs deliberately don't need. `Config` already derives `clap::Parser`,
so the verb reuses it rather than restating the flags.

The `askance-server` binary still builds after this task. Retiring it is task
02, so this one is judged on `askance serve` working, not on anything being
gone.

The verb also takes over the logging setup the server binary did on its way in:
the `tracing_subscriber` fmt layer with an `EnvFilter` defaulting to
`askance_server=info`, honouring `RUST_LOG`. **Decided in planning:** the CLI
takes its own `tracing-subscriber` dependency for this rather than the server
lib exporting an init function or `run` doing it itself.

The agent-facing contract must not move: `askance ask` and `askance guide` keep
their exact behaviour, stdout discipline included — the Response and the Guide
are still the only things on stdout, and bare `askance` still prints the Guide
rather than a usage error.

## Acceptance criteria

- [ ] `askance serve` binds `127.0.0.1:8422` by default and serves both the
      agent API and the viewer, and `askance ask` against it round-trips
- [ ] `--listen` and `--database` work under the verb, as do `ASKANCE_LISTEN`
      and `ASKANCE_DATABASE`, with the same defaults as before
- [ ] `askance serve --help` describes the flags; bare `askance` still prints
      the Guide, and `askance ask` / `askance guide` are unchanged
- [ ] Startup logs the listen address and database path as before, and
      `RUST_LOG` still overrides the default filter
- [ ] `cargo test` passes, `cargo clippy` is clean
