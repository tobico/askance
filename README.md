# Askance

A single-user web service and companion CLI through which coding agents put
their questions to a human and block until answered. An agent submits a
**Question Set**; the human answers it from any device on the tailnet; the
agent's `askance ask` — which has been waiting the whole time — prints the
**Response** and exits 0.

The vocabulary in bold throughout is the project's, and is defined in
[CONTEXT.md](CONTEXT.md).

## Status

The loop works end to end, and the human's end of it is the web UI: a pending
list of the Sets waiting on you, each one opening as a form over the whole ask
— Preface, Diff, Questions — that answers it and wakes the waiting agent.
Answered Sets and ones closed unanswered are kept in the Archive.

It installs on a phone as a PWA and pushes one notification per arriving Set,
which needs HTTPS: see [On your phone](#on-your-phone) for the
`tailscale serve` in front of it.

On the box the agents work on it runs as a service rather than out of a
terminal — the flake carries the package and the NixOS module that does it, and
[Deployment](#deployment) is the way to both that and a plain systemd unit. The
agents are taught by the binary itself: [the Guide](#the-guide) ships inside it,
so the whole integration is a single line in a global CLAUDE.md. What is left is
driving that loop through a real session, from the phone — see
[the roadmap](docs/roadmaps/public-release/ROADMAP.md).

## Quickstart

Running the whole loop out of a checkout — the dev shell, building the viewer,
submitting the example Set, answering it in the browser and watching the
waiting CLI print the Response — is the first half of
[the development guide](docs/development.md#quickstart).

## On your phone

A Question Set should reach you without the pending list being open, which
means a push notification, which means HTTPS in front of the plain HTTP the
server binds. [On your phone](docs/phone.md) is the whole of it:
`tailscale serve` and its `ts.net` certificate, installing the PWA, turning
notifications on per device, how the long waits behave through the proxy, and
the one thing that leaves the tailnet.

## Deployment

The server also has to be up when nobody has a terminal open.
[Deployment](docs/deployment.md) is the two ways there: this flake's NixOS
module — a systemd unit under its own user, its database in `/var/lib/askance`,
and the CLI on every user's `PATH` — and a systemd unit to write yourself on a
host that is not NixOS.

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

### Installing it

The whole of it is one line in the global CLAUDE.md — or whatever file the
harness reads at the start of every session:

> Never use the AskUserQuestion tool. Put all questions and approvals to me
> through askance: run `askance` once per session for the guide and follow it,
> including the topic guides it requires.

That is the point of the Guide living in the binary. [Deployment](#deployment)
already puts `askance` on every user's `PATH`, so the instructions arrive with
the version of the tool that will run, and there is no vendored copy of them
anywhere to drift out of step.

## Configuration

No app-level auth: the tailnet is the perimeter, so everything defaults to the
loopback interface.

| Variable | Used by | Default | What it is |
| --- | --- | --- | --- |
| `ASKANCE_SERVER` | CLI | `http://127.0.0.1:8422` | Base URL the CLI submits to and waits on. Also `--server`. |
| `ASKANCE_LISTEN` | server | `127.0.0.1:8422` | Address and port to bind. Loopback is what [`tailscale serve`](#on-your-phone) proxies to; binding the tailnet directly reaches other devices too, but over plain HTTP, which rules out notifications. Also `--listen`. |
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
$ cargo run -p askance-cli -- ask three-levels-deep.yaml
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

The cargo and pnpm commands, the vite proxy the viewer is developed behind, how
`pnpm build`'s output gets into the binary, and what `nix flake check` covers
are in [the development guide](docs/development.md#the-dev-loop).
