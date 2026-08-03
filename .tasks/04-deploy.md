# 04. Deploy it on the real box

## What to build

Askance running as a service on the machine the agents actually work on, reached
over `tailscale serve` from the phone, and the documentation a host needs to get
there.

The host's configuration is a separate flake, so this task spans two repositories.
Askance is a private repo, and the host takes it as a `github:` flake input like
its other external inputs — which means nix needs an access token to fetch it, and
the deployment does not work without one. Configure that alongside the input;
whatever a fresh `nixos-rebuild` on this host needs is part of the deliverable,
not a footnote.

Only the documentation lands in this repository. Leave the host config edits in
place, uncommitted, for review — they belong to that repo's history, not this
feature's.

Then verify it as the deployment it is, not as a build that evaluated: rebuild,
reboot, and put a real `askance ask` through the deployed service from a real
repository, answering it from the phone over the `ts.net` URL. If the pushed
notification arrives too, say so — that is the whole loop finally running as a
service rather than out of a terminal.

The README's Quickstart stays the from-a-checkout story. The deployment gets its
own section: the flake input, the module import, the rebuild step, and a pointer
to the `tailscale serve` invocation already documented under "On your phone"
rather than a second copy of it. The Status section should also stop describing
Askance as something you run in a terminal, if that is what it still says.

## Acceptance criteria

- [ ] The service runs on the host from the flake, and comes back by itself after
      a reboot
- [ ] An `askance ask` against the deployed service round-trips: answered from the
      phone over the `ts.net` URL, the Response reaches the waiting agent
- [ ] The private-repo access the flake input needs is configured on the host and
      written down
- [ ] The README documents deployment as its own step — input, module, rebuild —
      and points at the existing `tailscale serve` section instead of repeating it
- [ ] The host config changes are left uncommitted in their own repository, and
      the working tree here contains only documentation
