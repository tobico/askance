# Askance

Your coding agent has reached a question only you can settle — which of two
designs, whether this is worth committing — and you are not at the terminal.
Its choices are to guess or to stall. Askance is the third one: the agent
submits a **Question Set** and blocks on it, you answer from your phone
whenever you next pick it up, and the `askance ask` that has been waiting the
whole time prints the **Response** so the agent can carry on. A wait of hours
is the tool working rather than the tool failing.

It is one binary. The same file is the server — the API agents submit to and
the web UI you answer in — and the CLI they call. The UI installs on a phone as
a PWA and pushes one notification per arriving Set. There is no app-level auth
anywhere in it, because the tailnet is the perimeter: the server binds loopback,
and [`tailscale serve`](#securing-access) is the only thing that listens.

Getting there is five steps, and this page is all of them — install the binary,
run the server, tell your agent about it, hand it the two skills, put Tailscale
in front. The vocabulary in bold throughout is the project's, and is defined in
[CONTEXT.md](CONTEXT.md).

## Installing the binary

One file, with the viewer inside it and statically linked on Linux, so it runs
on any distribution. This fetches the build for your platform into
`~/.local/bin`:

```console
$ mkdir -p ~/.local/bin && curl -fsSL -o ~/.local/bin/askance \
    "https://github.com/tobico/askance/releases/latest/download/askance-$(uname -s | tr '[:upper:]' '[:lower:]' | sed s/darwin/macos/)-$(uname -m | sed -e s/x86_64/x64/ -e s/aarch64/arm64/)" \
    && chmod +x ~/.local/bin/askance
$ askance --version
askance 0.1.0
```

It asks for `releases/latest/download` rather than a version, so the command
above is the same one next year. The `uname` pair picks between the four assets
a release publishes — `askance-linux-x64`, `askance-linux-arm64`,
`askance-macos-x64` and `askance-macos-arm64` — and any of them can be fetched
by name instead. `~/.local/bin` has to be on your `PATH`, which on most systems
it already is.

### With nix

```console
$ nix run github:tobico/askance          # the server
$ nix run github:tobico/askance#askance  # the CLI
```

Both run the same released asset the curl above fetches, downloaded and
hash-checked rather than compiled; `askance-source` is the attribute that builds
this tree instead. For an install that persists, take the flake as an input:

```nix
inputs.askance = {
  url = "github:tobico/askance";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

On NixOS the input also carries a module that runs the server as a service —
[Deployment](docs/deployment.md#nixos) is that in full.

## Running the server

```console
$ askance serve
```

It binds `127.0.0.1:8422` and keeps its SQLite database in `askance.db` beside
you, both movable — see [Configuration](#configuration). Open
<http://127.0.0.1:8422/> and there is the pending list, empty for now: every Set
waiting on you appears there, opening as a form over the whole ask — Preface,
Diff, Questions — that answers it and wakes the agent. Answered Sets, and ones
closed unanswered, are kept in the Archive.

An agent is blocked for as long as the server is down, so it wants to be up when
nobody has a terminal open. [Deployment](docs/deployment.md) is the two ways
there: this flake's [NixOS module](docs/deployment.md#nixos), which puts the
service under its own user and the CLI on every user's `PATH`, and
[a systemd unit](docs/deployment.md#anywhere-else-a-systemd-unit) to write
yourself on a host that is not NixOS.

## Configuring your agent

The whole of the integration is one line in the global `CLAUDE.md` — or
`AGENTS.md`, or whatever file your harness reads at the start of every session:

> Never use the AskUserQuestion tool. Put all questions and approvals to me
> through askance: run `askance` once per session for the guide and follow it,
> including the topic guides it requires.

Drop the first sentence if your harness has no such tool of its own. One line is
enough because the instructions ship inside the binary: [the Guide](#the-guide)
is what `askance` prints when it is run with no arguments. So an agent learns to
ask from the version of the tool it will actually call — [the binary you just
installed](#installing-the-binary), or the one the NixOS module put on its
`PATH` — and there is no vendored copy of the instructions anywhere to drift out
of step.

## Skills

Two habits are worth teaching alongside the tool, and
[`examples/skills/`](examples/skills/) carries both as plain markdown, belonging
to no particular harness: paste one into a skills directory, or into the
instructions file above.

**[Grilling](examples/skills/grilling.md)** — interview the human about a plan
until you both understand it the same way, instead of discovering the
disagreement a week into building it. What keeps that from spending their
attention on your work:

> - **Ask only about what the human can settle and you can't**: trade-offs decided
>   by taste, product or cost; assumptions nothing in the repository confirms;
>   scope; facts from outside the codebase.
> - **If a question can be answered by exploring the codebase, explore the
>   codebase instead.** A question you were supposed to answer yourself spends
>   their attention on your work.

**[Acceptance gate](examples/skills/acceptance-gate.md)** — stop before the
commit, the push, the deploy, say what you did, and ask the one question a gate
is. The strictness is the point of it:

> Only an explicit approval opens the gate. Everything else leaves it shut:
>
> - **No answer to the gate itself** — shut, even where everything around it was
>   answered.
> - **A question back, or a partial answer** — a counter-question, not approval.
>   Answer it, then put the same gate again.
> - **Anything ambiguous** — shut. Ask again rather than resolving it in your own
>   favour.

The binary teaches the same thing from the other end: `askance guide gates` is
required reading before an agent writes a gate, and says how a Set that asks for
approval is authored and how its Response is read.

## Securing access

Askance has no login page. What it holds — every Set, every Diff of your work in
progress, every answer — sits behind the tailnet instead, and the server binds
loopback so that nothing reaches it even from the tailnet except through what
you put in front.

That is `tailscale serve`, which also settles the other half: notifications need
HTTPS, because service workers and the Push API are withheld outside a secure
context and a plain `http://` tailnet address is not one.

```console
$ tailscale serve --bg 8422
Available within your tailnet:

https://your-host.your-tailnet.ts.net/
|-- proxy http://127.0.0.1:8422
```

`--bg` is what makes it persist across a reboot. That URL is the one to open on
the phone, and [On your phone](docs/phone.md) is the rest of it: adding it to the
home screen, turning notifications on per device, and how the long waits behave
through the proxy.

**Not `tailscale funnel`.** The sibling command would put the same service on the
public internet, where nothing in Askance stops whoever arrives. Tailnet only.
The single exception is outbound and unavoidable — a notification is delivered
by Apple's or Google's push service, so
[the server reaches out](docs/phone.md#the-one-thing-that-leaves-the-tailnet) to
one of those, carrying a payload it cannot read. Nothing reaches in.

## Updating

`askance --version` says what you are running, and the pending list says when
that is behind: the server asks GitHub once a day whether a newer Askance has
been released, and puts a banner above the list when one has. It only ever says
so — nothing is installed for you, here or anywhere else.

**Installed with curl:** run the install command again, which overwrites the
binary in place, then restart the server — `systemctl restart askance` where it
runs as a service.

**Installed from the flake:** `nix flake update askance` in your host
configuration, then rebuild. The input tracks this repository rather than a
release, so what moves is [`nix/release.json`](nix/release.json) — the version,
url and hash the release workflow commits after every tag, and the only thing the
package reads to decide which binary to fetch.

The database is untouched by either, so the Archive and the phone's push
subscription come back with the new binary. To stop the daily check, and the
banner with it, set `ASKANCE_NO_UPDATE_CHECK` — or
`services.askance.updateCheck = false` on NixOS.

## The Guide

An agent that has never seen Askance still has to write a Set worth answering,
run the CLI so the wait outlives its own tool timeout, and read the Response
strictly enough not to invent an approval nobody gave. All of that ships inside
the binary:

```console
$ askance guide
```

Bare `askance` prints the same thing rather than clap's usage error, so an agent
that runs the command to find out what it is gets the instructions. Either way
it is markdown on stdout and exit 0.

The core Guide is everything any ask needs: how Questions, Sub-questions and
Options are labelled and how the Recommendation is marked, how much to put in
one Set and how fast to ask, running `askance ask` as a background command
because the human is not at the terminal, and reading every field that can come
back — `unanswered: true` above all, which means still open and never acceptance
of the Recommendation.

Then the Topics. A **Topic** is part of the Guide rather than an appendix to it,
split out only so an agent pays its reading cost when the task that needs it
arrives instead of before every ask:

```console
$ askance guide gates
```

`gates` is the one Topic so far, and it is required reading before an agent
writes a **confirmation gate** — the degenerate Set, one question and a yes,
that asks for approval to commit or to land. A gate is authored differently from
an ordinary Set and its Response is read far more strictly, which is a lot of
words to carry into every ask that is not one, and expensive to be missing at
the moment it is: the cost of getting a gate wrong is work landing that nobody
approved. The core Guide names the Topic and says when it is mandatory, so an
agent that has read only the core still knows what it has not read yet.

The text is markdown in this repository — [`crates/cli/guide/`](crates/cli/guide/)
— embedded at compile time rather than assembled at run time, so what an agent
reads is exactly what was reviewed and the binary alone is the documentation. A
Topic that does not exist is an error naming the ones that do, never a quiet
fallback to the core: an agent that asked for required reading must not be
handed something else.

## Configuration

No app-level auth: the tailnet is the perimeter, so everything defaults to the
loopback interface.

| Variable | Used by | Default | What it is |
| --- | --- | --- | --- |
| `ASKANCE_SERVER` | CLI | `http://127.0.0.1:8422` | Base URL the CLI submits to and waits on. Also `--server`. |
| `ASKANCE_LISTEN` | server | `127.0.0.1:8422` | Address and port to bind. Loopback is what [`tailscale serve`](#securing-access) proxies to; binding the tailnet directly reaches other devices too, but over plain HTTP, which rules out notifications. Also `--listen`. |
| `ASKANCE_DATABASE` | server | `askance.db` | SQLite file, created with its parent directory. Also `--database`. |
| `ASKANCE_NO_UPDATE_CHECK` | server | unset | Set it to stop the server asking GitHub, once a day, whether a newer Askance has been released — and so to stop the banner that says one has. Nothing is ever installed either way. Also `--no-update-check`. |

## The wire format

YAML in both directions. The types are defined — and documented field by field
— in the `askance-schema` crate: the Set in
[`crates/schema/src/set.rs`](crates/schema/src/set.rs), the Response in
[`crates/schema/src/response.rs`](crates/schema/src/response.rs), and the
grammar both ends enforce in
[`crates/schema/src/validate.rs`](crates/schema/src/validate.rs).

### Question Set

An agent supplies `title`, `preface`, `questions` and `postscript`. It never
supplies `project`, `branch` or `diff`: the CLI derives those from the working
directory and overwrites anything a Set claims.

```yaml
title: Rate limiting for the public API   # required, and plain text
preface: |                                # optional markdown; everything the
  Why this needs deciding, in enough      #   human needs without seeing the
  detail to answer without asking back.   #   agent's session
questions:
  - label: Q1                             # agent-owned and opaque; the Response
                                          #   answers by this name
    text: Where should the counter live?  # markdown, blocks and all
    options:                              # optional
      - n: 1                              # the number the human selects
        text: In-process, per instance.   # inline markdown only
        recommended: true                 # the Recommendation; at most one
      - n: 2                              #   per question
        text: In Redis, shared.
    subquestions:                         # optional, and leaves: no third level
      - letter: a                         # named Q1a — parent's label + letter
        text: What should Retry-After say?
        options: [...]                    # same shape as a Question's
postscript: |                             # optional markdown, drawn above the
  Where "anything else?" goes instead     #   set-level comment box. Never a
  of a Question — open invitations        #   Question: an empty box under it
  only, never a decision.                 #   means nothing to add
```

The rest of the grammar: a Set needs a non-empty title, labels are distinct
across the Set, Option numbers are distinct within a question, and there is no
multi-select — an Answer carries one `selected`.

Markdown is the agent's half of the page. The `preface` and the `postscript`,
and the `text` of a Question and of a Sub-question, are rendered in full:
headings, lists, tables, fenced code. An Option's `text` gets inline markup
only — the emphasis, the code spans, the links — because an Option is one line
beside a radio, and a block written there is flattened into that line rather
than drawn as one. The `title` is not markdown: it heads the page and stands in
the pending list and the Archive as typed.

All of it is rendered on the server and sanitized on the way through, so a
script or an event handler written into any of those fields is dropped rather
than run, and no markdown parser reaches the browser.

### Diagrams

A ```` ```mermaid ```` fence in any of those four fields is a **Diagram**: the
structural part of a Preface said as a picture, for the Set that is quicker to
grasp as one. It is the one thing on the page mermaid draws rather than the
server, and the bundle that draws it is imported only by a Set that carries one
([ADR 0003](docs/adr/0003-solid-spa-viewer.md)).

It is therefore also the one thing on the page that can fail to render, so it
degrades rather than breaking: a fence mermaid cannot parse and a bundle that
never arrived both leave the fenced source exactly as the agent wrote it, and
never an error graphic. Write a diagram whose source reads as text, because
sometimes it is read as text.

Three node classes are themed for marking a delta — which is what a Diagram at
a code approval gate is usually for. Tag a node `new`, `modified` or `removed`
and it takes the matching colour from the Diff's own palette, in the light
scheme and the dark one alike, so the picture rhymes with the Diff underneath
it. An untagged node keeps the base theme, which is what makes a marked one
read as marked:

````yaml
preface: |
  The counter moves out of the process, the throttle goes away, and the
  endpoint gains a limiter in front of it.

  ```mermaid
  flowchart LR
    api[POST /v1/messages] --> limiter[Rate limiter]
    limiter --> handler[Handler]
    limiter --> counter[(Redis counter)]
    handler --> throttle[In-process throttle]

    class limiter,counter new
    class api modified
    class throttle removed
  ```
````

### Response

One entry per Question **and** Sub-question in the Set — for the example
above, `Q1` and `Q1a`. The invariant is explicitness, not completeness: a
question may be left open, but it has to be said out loud, so the agent can
never silently miss one.

```yaml
answers:
  - label: Q1              # the question this resolves
    selected: 1            # an Option the question actually offers,
    free_text: Start small #   and/or the human's own words
  - label: Q1a
    unanswered: true       # left open on purpose; exclusive with an Answer
comment: |                 # optional, about the Set as a whole
  A Response of nothing but `unanswered` entries plus a comment is a valid
  counter-question: the agent has to take the discussion back a step.
```

The human's half is plain text throughout. `free_text` and `comment` are their
own words, and come back as typed — line breaks and all, and read back on the
archived page the same way. Nothing in a Response has been through a markdown
parser, in either direction.

### Refusals

Both ends enforce the grammar. The CLI checks a Set before sending it, so a
bad Set never reaches the server:

```console
$ askance ask three-levels-deep.yaml
askance: the Question Set breaks the question grammar:
  Q1a: Sub-questions are leaves: two levels is the maximum, so this one cannot have Sub-questions of its own
```

The server refuses the same things the same way — 400 for YAML that is not a
Set or a Response, 422 for one that breaks the grammar, listing every
violation and naming the question at fault:

```console
$ curl -X POST --data-binary @incomplete.yaml \
    -H 'Content-Type: application/yaml' \
    http://127.0.0.1:8422/api/v1/sets/2/response
error: the Response does not resolve the Question Set
violations:
- label: Q2
  message: "missing from the Response; every question appears, either answered or marked `unanswered: true`"
```

A Set is answered once: a second Response is a 409 and the first one stands.

## API

| | |
| --- | --- |
| `GET /api/v1/health` | `ok`. |
| `POST /api/v1/sets` | A Question Set in, `201` with its `id` and `created_at` back. |
| `GET /api/v1/sets/{id}/response?hold={seconds}` | The wait. `200` with the Response, or `204` — "nothing yet, come back" — once the hold window closes. `hold` is clamped to 60s; the client owns retry. |
| `POST /api/v1/sets/{id}/response` | The human's Response in, `201` with `set_id` and `submitted_at` back. Wakes every wait held on the Set. |

The viewer has a namespace of its own under `/api/ui/` — the pending list, the
Archive, one Set, answering it, archiving it, the three push endpoints, and the
Nudge stream an open page listens on. It is private to the viewer that ships in
the same binary and is not part of the contract above: it speaks JSON rather
than YAML, and it may be rearranged whenever the viewer is. Agents use
`/api/v1/` alone.

Everything outside `/api/` is the viewer: its own files where it has them, and
the document everywhere else, because the viewer routes in the browser and
`/sets/12` is a path only it knows. A *file* that is not there is still a `404`
— a missing bundle answered with HTML would die as a syntax error at the top of
the page rather than reporting a stale URL.

## Development

Running the whole loop out of a checkout — the dev shell, building the viewer,
submitting the example Set, answering it in the browser and watching the waiting
CLI print the Response — is
[the development guide's quickstart](docs/development.md#quickstart). The cargo
and pnpm commands, the vite proxy the viewer is developed behind, how
`pnpm build`'s output gets into the binary, and what `nix flake check` covers
are [the dev loop](docs/development.md#the-dev-loop) after it. Shipping what
comes out of it is [Releasing](docs/releasing.md): what a `v*` tag sets off, and
what is left to check by hand once it has. What is still ahead is
[the roadmap](docs/roadmaps/public-release/ROADMAP.md).
