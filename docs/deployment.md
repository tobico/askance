# Deployment

Answering from the phone is only half of it: the server also has to be up when
nobody has a terminal open. This guide is the two ways of getting there — the
NixOS module this flake carries, and a systemd unit for a host that is not
NixOS.

Either way the server binds loopback and speaks plain HTTP. HTTPS is
[`tailscale serve`](phone.md#1-put-tailscale-serve-in-front-of-it) in front of
it, and stays host-level configuration in both.

## NixOS

On the NixOS box the agents work on, that is this flake's NixOS module — a
systemd unit under its own user, its database in `/var/lib/askance`, and the
CLI on every user's `PATH` so an agent just calls `askance`. It is the same
package `nix run` gives you, so the host needs no checkout of this repository
at all.

Two steps, in a host's own flake-based configuration.

### 1. Add the flake input

```nix
inputs.askance = {
  url = "github:tobico/askance";
  # Askance tracks a nixpkgs release; following the host's own input keeps a
  # second copy of it out of the lock file.
  inputs.nixpkgs.follows = "nixpkgs";
};
```

The input carries two packages. `askance` is the default — what an import that
names no attribute gets, and what the module runs — and it downloads the binary
the release workflow already built for the host's system, so nothing here needs
a Rust toolchain and `nixos-rebuild` does not turn into a workspace compile.
`askance-source` builds from this tree instead, for anyone who would rather
compile than trust an asset; it is what `nix flake check` proves, and
`services.askance.package` takes it as readily as the default.

### 2. Import the module and enable it

The module is a function of this flake rather than of `pkgs` — the package is
not in nixpkgs to be found by name — so it comes from the input, not from a
path:

```nix
nixosConfigurations.your-host = nixpkgs.lib.nixosSystem {
  system = "x86_64-linux";
  modules = [
    askance.nixosModules.askance
    { services.askance.enable = true; }
    ./configuration.nix
  ];
};
```

Then lock the new input and rebuild:

```console
$ cd /path/to/host-config && nix flake lock
$ sudo nixos-rebuild switch --flake /path/to/host-config#your-host
$ systemctl status askance
● askance.service - Askance — questions from coding agents to a human
     Active: active (running)

$ curl http://127.0.0.1:8422/api/v1/health
ok
```

The unit is wanted by `multi-user.target` and restarts always, so it comes up
on boot and comes back after a crash — an agent is blocked on an answer
whenever the server is down, which is the whole reason for both. The database
lives in the service's state directory, so reboots and rebuilds keep the
Archive and the phone's push subscription with it.

Updates reach the host the way any other input's do: `nix flake update askance`
in the host configuration, then rebuild. The input still tracks the repository
rather than a release, so what moves is
[`nix/release.json`](../nix/release.json) — the version, url and hash the
release workflow commits to `main` after every tag, and the only thing the
default package reads to decide which binary to fetch.

### What it leaves to the host

**HTTPS.** The service binds loopback and speaks plain HTTP, exactly as it does
from a checkout, and `tailscale serve` is still the thing in front of it — see
[Put `tailscale serve` in front of it](phone.md#1-put-tailscale-serve-in-front-of-it).
The module deliberately keeps no second copy of that invocation, and `--bg`
means it survives the reboot alongside the service.

**The two paths**, if the defaults do not suit: `services.askance.listen` and
`services.askance.database` are the module's spellings of `ASKANCE_LISTEN` and
`ASKANCE_DATABASE` in [Configuration](../README.md#configuration). A port other
than the default also means giving the agents `ASKANCE_SERVER`, since the CLI's
own default is `http://127.0.0.1:8422` and it does not learn otherwise from the
module.

**The update check**, which is on by default:
`services.askance.updateCheck = false` is the module's spelling of
`ASKANCE_NO_UPDATE_CHECK`, and turns off the one request this service makes
that is not a notification.

## Anywhere else: a systemd unit

Without the module, the same shape is a unit file you write once. The Linux
release assets are statically linked, so the whole of the install is putting
one somewhere on `PATH` — [the README](../README.md#installing-the-binary) has
the download — and
pointing a unit at it. The rest of this section assumes
`/usr/local/bin/askance`.

Give the service a user of its own, so the database and the VAPID keypair are
not readable by everything on the box:

```console
$ sudo useradd --system --user-group --no-create-home \
    --shell /usr/sbin/nologin askance
```

Then `/etc/systemd/system/askance.service`:

```ini
[Unit]
Description=Askance — questions from coding agents to a human
After=network.target

[Service]
ExecStart=/usr/local/bin/askance serve --listen 127.0.0.1:8422 \
    --database /var/lib/askance/askance.db

User=askance
Group=askance

# systemd creates /var/lib/askance, owned by the service user, before the first
# start and leaves it there across restarts — which is what keeps the Archive
# and the push subscriptions. Relative paths resolve here too.
StateDirectory=askance
StateDirectoryMode=0750
WorkingDirectory=/var/lib/askance

# An agent is blocked on an answer whenever the server is down, so come back
# rather than sit in a failed state.
Restart=always
RestartSec=5s

# Enough hardening to be worth having, without breaking the two things that
# matter: SQLite in WAL mode, which writes `-wal` and `-shm` beside the
# database and so needs a writable directory rather than a writable file, and
# outbound HTTPS to the push services, whose addresses cannot be enumerated
# ahead of time.
NoNewPrivileges=true
PrivateTmp=true
ProtectHome=true
ProtectSystem=strict
ProtectKernelTunables=true
ProtectControlGroups=true
RestrictSUIDSGID=true
UMask=0077

[Install]
WantedBy=multi-user.target
```

`ExecStart` passes the flags rather than setting the environment variables
behind them, so `systemctl cat askance` says what this service is actually
running. Where the environment is the better fit — a secret, or a value managed
elsewhere — `EnvironmentFile=/etc/askance.env` reads
[the same variables](../README.md#configuration) out of a `KEY=value` file, and
a flag on `ExecStart` wins over the variable it shadows — so drop from
`ExecStart` whatever the file is to set. The file is plain `KEY=value` lines,
readable by the service user and nobody else (`chown root:askance`,
`chmod 0640`):

```sh
ASKANCE_LISTEN=127.0.0.1:8422
ASKANCE_DATABASE=/var/lib/askance/askance.db
```

Start it and check the same two things the NixOS path checks:

```console
$ sudo systemctl daemon-reload
$ sudo systemctl enable --now askance
$ systemctl status askance
● askance.service - Askance — questions from coding agents to a human
     Active: active (running)

$ curl http://127.0.0.1:8422/api/v1/health
ok
```

The CLI is the same binary, already on `PATH` from the install, so an agent on
this host calls `askance ask` with nothing set in its environment. Updating is
replacing the file and restarting the unit; the database is untouched by both.
