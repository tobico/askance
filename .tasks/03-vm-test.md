# 03. A VM test of the module

## What to build

A NixOS VM test, wired into `nix flake check`, that boots a machine with the
module enabled and puts a Question Set through it end to end: the CLI asks and
blocks, the Response is submitted over the API the web UI posts through, and the
CLI prints it and exits 0. That is the same round trip the crate-level tests
cover in-process — the point of doing it again in a VM is everything the module
adds around it, which no in-process test can see.

What the VM is there to prove:

- the service comes up on its own at boot, with no manual start;
- the database is created in the state directory, by the service's user;
- state survives — a Set submitted before a service restart is still pending
  after it, and an agent waiting across the restart recovers rather than failing
  (the CLI reconnects its wait, so this should hold; if it does not, the test
  records what actually happens and the fix is a separate decision);
- the CLI on the host's `PATH` reaches the server with no environment set.

Push delivery stays out of scope: it needs the vendors' push services, and a VM
with no route to the internet cannot exercise it. The unit tests already cover
what gets sent.

Keep the test cheap enough to run on the way past — one machine, no wasm rebuild
beyond what the package already produced.

## Acceptance criteria

- [ ] `nix flake check` runs the test and it passes on `x86_64-linux`
- [ ] The test asserts the service is up after boot without intervention, and that
      the database exists in the state directory owned by the service user
- [ ] An `askance ask` inside the VM blocks, receives a Response submitted through
      the API, prints it and exits 0
- [ ] The test covers a service restart with a Set pending, and asserts what
      happens to an agent waiting across it
- [ ] The check is skipped or absent, rather than failing, on systems where a
      NixOS VM test cannot run
