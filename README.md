# Askance

Askance is simple and beautiful human-in-the-loop GUI for answering questions
from an AI coding agent such as Claude Code, either in the browser or on the go
with your phone.

It works by combining a server that you run locally and access from your phone
via Tailscale, and a small self-documenting command line utility which your
agent uses to put questions to you. The CLI blocks on your answer so your agent
waits on you, and you can answer whenever you're ready without wasting any
tokens.

## Getting started

### Installation

#### User binary

Askance is distributed as a single binary, which you download and run from your
home folder. It runs on Linux, Mac, and Windows via WSL.

```console
mkdir -p ~/.local/bin && curl -fsSL -o ~/.local/bin/askance \
  "https://github.com/tobico/askance/releases/latest/download/askance-$(uname -s | tr '[:upper:]' '[:lower:]' | sed s/darwin/macos/)-$(uname -m | sed -e s/x86_64/x64/ -e s/aarch64/arm64/)" \
  && chmod +x ~/.local/bin/askance
```

You can run the server manually with `askance serve`, by default it will listen
on '127.0.0.1:8422". I recommend setting up systemd config to run it
automatically on system start:

EXAMPLE GOES HERE

#### Nix flake

Add the following to your nix flake to install fully on nix.

```nix
# Imports the latest version as a nix flake
inputs.askance = {
  url = "github:tobico/askance";
  inputs.nixpkgs.follows = "nixpkgs";
};

# Enables the askance server daemon and CLI utility.
services.askance.enable = true;
```

### Configuring your agent

Add the following to your `~/.claude/CLAUDE.md` file or equivalent:

> Never use the AskUserQuestion tool. Put all questions and approvals to me
> through askance: run `askance` once per session for the guide and follow it,
> including the topic guides it requires.

The `askance` CLI is self-documenting and will guide your agent with
built-in guides on how to use it effectively as needed.

## Skills

Askance is designed to be especially useful in two sitations:

1. When your agent asks you a lot of questions
2. When your agent leaves code uncommitted as an acceptance gate, asking your
  permission before committing them.

In order to benefit from this, you need to actually have skills or prompts which
instruct your agent to do those things.

Two such skills are included as examples, but feel free to use whichever skills
or prompting approach you prefer, Askance will adapt to it.

**[Grilling](examples/skills/grilling.md)** — interviews you about a plan by
  asking a series of questions until a shared understanding is reached.
  Inspired by the [Matt Pocock grilling skill](https://github.com/mattpocock/skills)

**[Acceptance gate](examples/skills/acceptance-gate.md)** — puts a gate before
  commiting code changes. The agent waits for your feedback and addresses it,
  only proceeding to commit the code with your express approval.

## Securing access

When running Askance, it's vital to understand the security implications. It
runs as a server on your machine, with no authentication system, and accepts
text input which is passed directly to your agent without sanitization.

What keeps it safe is making sure that your server is never made available to
others. By default it runs bound to 127.0.0.1, meaning it can't be accessed
from other machines.

If you do want to access it from another machine (your phone for example), the
recommended approach is to proxy that connection through `tailscale serve`,
which lets you use Tailscale's authentication to control access, and adds TLS
encryption provided by Tailscale (which is itself a requirement if you want to
enable push notifications).

With Tailscale installed, run the following to set up a Tailscale proxy to your
Askance server. The `--bg` option makes it persistent across reboots.

```console
tailscale serve --bg 8422
```

**Note:** — To receive notifications on iOS, you'll need to add the site to
your home screen. On Mobile Safari, tap "...", "Share", "View More", then
"Add to Home Screen".

## Updating

To check the current version, run `askance --version` 

**Installed with curl:** run the install command again, which overwrites the
binary in place, then restart the server.

**Installed from the flake:** Run `nix flake update askance` in your host
configuration, then rebuild. Then run `sudo systemctl restart askance` to
restart the server.

## Configuration

Askance can be configured by environment variables or CLI options:

| Variable | CLI Option | Used by | Default | What it is |
| --- | --- | --- | --- | --- |
| `ASKANCE_SERVER` | `--server` | CLI | `http://127.0.0.1:8422` | Base URL the CLI submits to and waits on. |
| `ASKANCE_LISTEN` | `--listen` | server | `127.0.0.1:8422` | Address and port to bind. |
| `ASKANCE_DATABASE` | `--database` | server | `askance.db` | SQLite database for Askance data. |
| `ASKANCE_NO_UPDATE_CHECK` | `--no-update-check` | server | unset | Disables the automated daily check to GitHub to notify you of a new Askance release. |

## Other documentation

LINKS TO DOCS HERE
