# 01. Docs guides extraction

## What to build

The technical depth currently in the README moves into guides under `docs/`,
and the README links to them. Nothing is cut: a reader who wants the detail
still reaches all of it, one hop further out.

Roughly what moves, by current README section:

- **Quickstart** (the dev shell, building the viewer, running from a checkout)
  and **Development** (the cargo and pnpm commands, the vite proxy note) become
  a development guide. These are for someone working *on* Askance, not someone
  adopting it.
- **Deployment** (the flake input, importing the NixOS module, what it leaves to
  the host) becomes a deployment guide.
- **On your phone** — the `tailscale serve` walkthrough, installing the PWA,
  turning notifications on, the long-waits-through-the-proxy timing notes, and
  what leaves the tailnet — becomes a guide of its own. The README keeps a short
  securing-access section in task 03; the depth lives here.

One piece is **new rather than moved**: a short systemd unit example for
daemonizing on a non-NixOS host, which the deployment guide gains alongside the
NixOS walkthrough. Today the only documented way to run Askance as a service is
the NixOS module.

The README is left structurally as it is otherwise — task 03 rewrites it. Here
each moved section's body is replaced by a link, so the repo is coherent at the
end of this task on its own.

## Acceptance criteria

- [ ] Every paragraph, command and caveat that leaves the README exists in a
      `docs/` guide — verified by reading the diff, not assumed
- [ ] The deployment guide carries a working systemd unit example for a
      non-NixOS host, including where the database lives and how the service
      gets its environment
- [ ] The README links to each new guide where its content used to be
- [ ] Every internal link in the repo resolves — no link points at a README
      anchor whose section has moved, and no guide points at a file that is not
      there
- [ ] `nix flake check` and `cargo test` still pass (nothing here should touch
      them, so a failure means something was moved that should not have been)
