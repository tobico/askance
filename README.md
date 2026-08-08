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

On the box the agents work on it runs as a NixOS service rather than out of a
terminal — the flake carries the package and the module that does it, and
[Deployment](#deployment) is the three steps to it. The agents are taught by the
binary itself: [the Guide](#the-guide) ships inside it, so the whole integration
is a single line in a global CLAUDE.md. What is left is driving that loop
through a real session, from the phone — see
[the roadmap](docs/roadmaps/v1/ROADMAP.md).

## Quickstart

The whole loop, from a fresh checkout. It takes two terminals: `askance ask`
blocks until it is answered, which is the entire point of it.

### 1. Enter the dev shell

```console
$ nix develop
```

Everything below assumes this shell — it carries the Rust toolchain, `sqlite`,
`git`, and the `node` and `pnpm` the viewer is built with.

### 2. Build the viewer and start the server (terminal 1)

```console
$ (cd web && pnpm install && pnpm build)
$ cargo run -p askance-cli -- serve
  INFO askance_server: askance is listening listen=127.0.0.1:8422 database=askance.db
```

One binary serves both halves: the agent API under `/api/v1/`, and the web UI
on <http://127.0.0.1:8422/>. It creates `askance.db` in the working directory
on first run. Leave it running; check it in a third terminal if you like:

```console
$ curl http://127.0.0.1:8422/api/v1/health
ok
```

The viewer is built into the binary, so `pnpm build` is what puts a UI at that
address. Skip it if only the API matters — the server starts either way, and
says on every page that the viewer was not built. While working on the viewer
itself, `pnpm dev` is the better half of this: see [Development](#development).

### 3. Ask (terminal 2)

```console
$ cargo run -p askance-cli -- ask examples/questions.yaml
```

A wait that goes to plan is silent, and the little the CLI does have to say —
reconnecting, or refusing a Set — is on **stderr**, written as a YAML comment.
Stdout carries the Response and nothing else, so an agent can parse it as it
stands, even out of the one file its harness merged both streams into.

The command does not return. It has submitted
[`examples/questions.yaml`](examples/questions.yaml) — along with the project
and branch it derived from this working directory, and the **Diff** of its
uncommitted changes if there are any — and is now holding a long-poll on
Question Set 1. There is no timeout: only an answer or a kill ends the wait
([ADR-0001](docs/adr/0001-blocking-cli-for-agent-integration.md)).

A Set can also arrive on stdin, which is how an agent usually sends one:

```console
$ cat examples/questions.yaml | cargo run -p askance-cli -- ask
```

### 4. Answer (in the browser)

This is the human's part. Open <http://127.0.0.1:8422/> and the Set from step 3
is on the pending list, with its project, its branch, and `agent waiting` —
that last one being the CLI still holding its long-poll. Open it.

The page is the whole ask: the Preface, the Diff of the working tree the agent
asked from, and each Question with its Options. Pick one, or write your own
words, or both — an Option with a ★ is the agent's Recommendation, and
**Accept all ★ Recommendations** fills in every question you have not answered
yet. Leave a question alone to send it back open; **Submit** asks you to
confirm that before it goes.

Under the last Question is the comment box, for anything about the Set as a
whole rather than about one Question — and directly above it, where the agent
wrote one, the **Postscript**: what it wanted to raise without making a
Question of it, which the example Set closes with. Nothing there has to be
answered, and an empty box says there was nothing to add.

The same Response can go in over the API instead, which is what an integration
test or a script does — see [the API](#api) and
[`examples/response.yaml`](examples/response.yaml), which answers every
Question in the example Set, leaves one explicitly open, and adds a set-level
comment.

### 5. Delivery

Back in terminal 2, the still-waiting CLI has printed the Response and exited —
this is what it prints for the answers in `examples/response.yaml`:

```console
answers:
- label: Q1
  selected: 1
  free_text: Start in-process. Revisit it the day we run more than two instances.
- label: Q2
  selected: 2
- label: Q2a
  unanswered: true
- label: Q3
  free_text: acct_8f21c3, the nightly export job in `ops/export`.
comment: |
  Ship it behind a flag and turn it on for the one noisy client first.

  That export job hammers the endpoint on purpose, so give it an allowlist
  entry rather than a bigger bucket for everyone.

  On Q2a I genuinely don't know — pick whatever our SDK's retry logic
  already understands, and say in the PR which one that was.

$ echo $?
0
```

That is the loop. Run step 3 again and Question Set 2 appears on the pending
list to be answered the same way. To answer it from your phone instead, carry
on below.

## On your phone

The point of putting Askance on a phone is that a Question Set reaches you
without the pending list being open. That needs a push notification, and a push
notification needs HTTPS: service workers, the Push API and
`Notification.requestPermission` are all withheld outside a secure context, and
`http://` over a tailnet is not one — only `localhost` gets the exemption. So
this section is HTTPS first, and everything else follows from it.

### 1. Put `tailscale serve` in front of it

`tailscale serve` terminates TLS with your tailnet's `ts.net` certificate and
proxies to the plain HTTP the server is already binding. Nothing about the
server changes: it stays on loopback, and the proxy is the only thing that
listens on the tailnet.

```console
$ tailscale serve --bg 8422
Available within your tailnet:

https://your-host.your-tailnet.ts.net/
|-- proxy http://127.0.0.1:8422

Serve started and running in the background.
To disable the proxy, run: tailscale serve --https=443 off
```

That URL is the one to open on the phone. It needs MagicDNS and HTTPS
certificates enabled for the tailnet — both are switches in the admin console,
under DNS — and `tailscale serve` says so if they are not.

`--bg` is what makes it persist: the configuration is stored in `tailscaled`'s
own preferences, so it survives a reboot and comes back with the daemon. Check
what is in force, or take it down again:

```console
$ tailscale serve status
https://your-host.your-tailnet.ts.net (tailnet only)
|-- / proxy http://127.0.0.1:8422

$ tailscale serve reset
```

This stays inside the tailnet. `tailscale funnel`, which is the sibling command
that would put the same service on the public internet, is not what you want
here: there is no app-level auth in Askance, and the tailnet is the whole
perimeter.

The CLI has no reason to go through the proxy — an agent runs on the same host
as the server and keeps talking to `http://127.0.0.1:8422`. Only the browser
needs the HTTPS URL.

### 2. Install it

On the phone, open the `ts.net` URL and add it to the home screen.

- **iOS/iPadOS** (16.4 or later): Safari, Share, **Add to Home Screen**. This
  is not optional — iOS gives Web Push only to a web app launched from the home
  screen, so in a Safari tab there is nothing to turn on, and the control on
  the pending list says notifications are unavailable. Open it from the home
  screen icon afterwards.
- **Android**: Chrome offers **Install app** from its menu. Push works in the
  tab too, but the installed app is what gets you an icon and no browser
  chrome.

Either way it opens standalone, without the address bar, on the pending list.

### 3. Turn notifications on

At the top of the pending list is a line saying where this device stands, with
one button. Tap **Turn on for this device** and answer the browser's permission
prompt.

This is per device, and it is read out of the browser on every load rather than
remembered — the phone being subscribed says nothing about the laptop, and an
app reopened a week later says what is actually true of it. **Turn off for this
device** is the way back. If it says notifications are *blocked*, the browser
has been told no and will not ask again: the way out is that browser's site
settings, not another tap.

On a Chromium browser that de-Googles — **Brave** above all — the tap can fail
with *Registration failed - push service error*. Chromium has no push transport
but Google's, and Brave ships with it switched off, so the subscribe is refused
inside the browser before this server hears about it. Turn on **Use Google
services for push messaging** under `brave://settings/privacy` and restart the
browser. That is the de-Googling trade this browser exists to offer, so it is
yours to make rather than something Askance can work around: Safari and Chrome
are unaffected, and so is Brave on Android once the same setting is on.

From then on, one notification per arriving Set — titled with the Set's own
title, with the project underneath it — and tapping it opens that Set, in the
Askance already on screen if there is one. There are no reminders: a Set that
goes unanswered is not notified about twice.

### The long waits, through the proxy

Two things here stay open much longer than a page load, and both go through
`tailscale serve` once the phone is the device answering: the CLI's wait, which
holds a request for up to a minute before reopening it, and the pending list's
ten-second refetch. Both already survive a dropped connection, so the question
was never whether the proxy breaks them but whether it makes them work harder
than they need to. It does not. Measured against tailscale 1.90.9:

- A full hold — 60 seconds, the server's ceiling — comes back `204` at 60.0s.
  The proxy neither cuts it short nor shortens the window it was asked for.
- A hold answered five seconds in comes back `200` at 5.0s. Nothing is
  buffered, which is what lets the phone's **Submit** wake a waiting agent
  immediately instead of at the end of whichever hold happened to be open.
- `askance ask` pointed at the `ts.net` URL and left for 75 seconds — three of
  its own 30-second holds — printed the Response and exited 0, having said
  nothing on stderr beyond the line it opens with. The reconnections are there
  and are invisible, which is the whole of what was asked of them.
- The pending list refetches as it does locally: a Set submitted while the
  installed app sits open arrives on the list without a touch.

The client end is HTTP/2 and the loopback hop is plain HTTP/1.1. The `ts.net`
certificate is publicly trusted, so nothing needs adding to a trust store —
which is the other reason this works on a phone at all.

### The one thing that leaves the tailnet

Web Push is delivered by the browser vendors' push services — Apple's, Google's
— so **the server needs outbound internet to send a notification**, even though
its inbound surface stays tailnet-only. That asymmetry is the whole of it:
nothing reaches Askance from outside the tailnet, and the only thing Askance
reaches out to is the push service for the device it is notifying, carrying an
encrypted payload it cannot read.

The VAPID keypair that signs those pushes is generated on first run and stored
in `askance.db`. There is no key ceremony and nothing to configure; a push
service that cannot be reached costs a notification, never a Question Set.

## Deployment

Answering from the phone is only half of it: the server also has to be up when
nobody has a terminal open. On the NixOS box the agents work on, that is this
flake's NixOS module — a systemd unit under its own user, its database in
`/var/lib/askance`, and the CLI on every user's `PATH` so an agent just calls
`askance`. It is the same package `nix run` gives you, so the host needs no
checkout of this repository at all.

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
rather than a release, so what moves is [`nix/release.json`](nix/release.json) —
the version, url and hash the release workflow commits to `main` after every
tag, and the only thing the default package reads to decide which binary to
fetch.

### What it leaves to the host

**HTTPS.** The service binds loopback and speaks plain HTTP, exactly as it does
from a checkout, and `tailscale serve` is still the thing in front of it — see
[Put `tailscale serve` in front of it](#1-put-tailscale-serve-in-front-of-it).
The module deliberately keeps no second copy of that invocation, and `--bg`
means it survives the reboot alongside the service.

**The two paths**, if the defaults do not suit: `services.askance.listen` and
`services.askance.database` are the module's spellings of `ASKANCE_LISTEN` and
`ASKANCE_DATABASE` below. A port other than the default also means giving the
agents `ASKANCE_SERVER`, since the CLI's own default is `http://127.0.0.1:8422`
and it does not learn otherwise from the module.

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

```console
$ cargo test              # unit, schema and end-to-end tests
$ cargo clippy --all-targets
$ cargo fmt
$ nix fmt                 # the Nix files
$ nix flake check         # the viewer's suite, and the NixOS module in a VM

$ tools/generate-icons.sh # the PWA icons, after editing their SVG
```

And in `web/`, which is the Solid viewer ([ADR
0003](docs/adr/0003-solid-spa-viewer.md)):

```console
$ pnpm install
$ pnpm dev                # the viewer on :5173, /api proxied to the server
$ pnpm test               # the vitest suite
$ pnpm typecheck          # tsc, which the tests do not run
$ pnpm build              # static assets, into web/dist
```

`pnpm dev` serves the viewer alone and proxies everything under `/api` to a
server on its usual `127.0.0.1:8422`, so the two run side by side in two
terminals and the browser sees one origin. The proxy is a development thing
only: the built assets are served by the server itself, out of the same binary.

Which is `pnpm build`'s output, embedded by rust-embed. A release build compiles
it in; a debug build reads it off disk per request, so a `cargo run -p
askance-cli -- serve` serves whatever `pnpm build` last wrote without a
recompile — and a checkout that has never built the viewer still builds the
server, which then says so on every page instead of serving one.

`cargo test` covers the round trip in-process. `nix flake check` runs the
viewer's vitest suite from the pinned pnpm and node, and boots a VM with the
NixOS module enabled to put a Question Set through it again, for the sake of
everything the module wraps around that round trip: a unit that starts itself
at boot, the state directory systemd hands over, a database that survives the
service being stopped and started under a waiting agent, a store-path binary
serving the viewer that was built into it, and the CLI on `PATH` with nothing
set in the environment. The VM needs a Linux host to boot the guest on, so on
macOS that half of the check is absent rather than failing.

The viewer's dependencies are fetched by a fixed-output derivation named by a
hash in [`nix/web.nix`](nix/web.nix) — move `web/package.json` or
`web/pnpm-lock.yaml` and that hash has to move too, and nix will print the one
it wanted. That file both builds the viewer and, with `runTests` on, is the
`nix flake check` above, so the two cannot disagree about a lockfile.

`assets/` is vite's `publicDir`, copied verbatim into the site root: the web
manifest, the icons and the service worker. They cannot live under `/assets/`
with the hashed bundles — a service worker only controls the paths beneath the
one it was served from, so one under the bundles' directory could never show a
notification for `/sets/12`, and the manifest and the icons keep the names the
phone knows them by. Which is also why the server keeps everything under
`/assets/` for a year and revalidates everything outside it.

The worker itself does no caching; every list and every Set is read from live
SQLite, and a cached copy of one that has since been answered is worse to the
human than a failure to load.

The icons are all one SVG, `assets/icons/askance.svg`, rasterized by the script
above (using `resvg` from the dev shell) to the PNG sizes the manifest and iOS
ask for. The PNGs are committed so a build needs nothing but cargo — edit the
SVG and re-run the script rather than touching them.

The tests run the real server in-process, so the round trip they check is the
one an agent gets — including the quickstart above, whose example files
[`crates/cli/tests/ask.rs`](crates/cli/tests/ask.rs) drives end to end, taking
the human's part over the API the viewer's **Submit** posts through.

`askance-render` is everything the server does to what an agent wrote before it
leaves: markdown to sanitized HTML, the Diff parsed and highlighted, and the view
types the viewer draws a Set from. It knows nothing of the store, the router or
the viewer, so it is the seam the browser never reaches across — everything past
it is HTML the viewer only has to put in the page.

Two things under `web/` are written by `cargo test` rather than by hand, and
both are committed so that the diff is the review. `web/src/api/types.ts` is
those view types as TypeScript, generated by ts-rs — the viewer imports them and
declares no shape of its own, so the two languages cannot come to disagree about
a field. `web/tests/fixtures/` holds a payload of each kind, rendered by the real
`/api/ui/` endpoints, which is what the vitest suite is fed: a component test
against a hand-written mock proves only that the mock and the component agree.
