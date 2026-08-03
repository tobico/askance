# 02. The NixOS module

## What to build

A NixOS module the flake exports, which runs the server as a systemd service:
enabled with one option, started at boot, restarted if it dies, with the CLI in
the system environment so an agent on the box can just call `askance`.

The options are the two the server already takes — the listen address and the
database path — named after them and defaulting to what the server itself
defaults to, so the module adds no third opinion about where Askance lives. Plus
the package, so a host can override it, and whatever `enable` is worth on top.

The service runs as a dedicated user with a state directory rather than as root
out of `/var/lib` by hand: systemd creates and owns the directory, and the
database default lands inside it. Hardening should go as far as it can without
breaking the two things the server genuinely needs — writing SQLite (with its
`-wal` and `-shm` companions, so a read-only or no-new-files sandbox is out) and
reaching the push services over the public internet, which is outbound HTTPS to
hosts that cannot be enumerated ahead of time.

Nothing here touches TLS or the tailnet. The service binds loopback and
`tailscale serve` sits in front of it, configured at the host level; the module's
documentation says so and points at the README rather than restating it.

## Acceptance criteria

- [ ] A host that sets the enable option and nothing else gets a running service
      whose database is inside its own state directory, owned by its own user
- [ ] The listen address and database path are settable, and each defaults to the
      server's own default
- [ ] `systemctl restart` leaves the database intact and the pending list
      unchanged
- [ ] The CLI is on `PATH` for a normal user on the host, pointing at the local
      server with no environment set
- [ ] The module evaluates for a host that only imports it, and the option
      descriptions say where the HTTPS story lives
