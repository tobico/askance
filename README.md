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
`tailscale serve` in front of it. What is left is packaging and the skills that
drive the tool — see [the roadmap](docs/roadmaps/v1/ROADMAP.md).

## Quickstart

The whole loop, from a fresh checkout. It takes two terminals: `askance ask`
blocks until it is answered, which is the entire point of it.

### 1. Enter the dev shell

```console
$ nix develop
```

Everything below assumes this shell — it carries the Rust toolchain, `sqlite`,
`git`, and `cargo-leptos` with the wasm tooling the web UI needs.

### 2. Start the server (terminal 1)

```console
$ cargo leptos watch
  INFO askance_server: askance is listening listen=127.0.0.1:8422 database=askance.db
```

One binary serves both halves: the agent API under `/api/v1/`, and the web UI
on <http://127.0.0.1:8422/>. It creates `askance.db` in the working directory
on first run. Leave it running; check it in a third terminal if you like:

```console
$ curl http://127.0.0.1:8422/api/v1/health
ok
```

`cargo run -p askance-server` also works, and is what you want when only the
API matters — it skips the wasm build and serves whatever `cargo leptos build`
last left in `target/site`.

### 3. Ask (terminal 2)

```console
$ cargo run -p askance-cli -- ask examples/questions.yaml
askance: Question Set 1 is waiting for an answer
```

That line is on **stderr**, and so is everything else the CLI has to say.
Stdout carries the Response and nothing else, so an agent can parse it as it
stands.

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
  free_text: |
    The nightly export job in `ops/export` hammers the endpoint on
    purpose. Give it an allowlist entry rather than a bigger bucket.
comment: |
  Ship it behind a flag and turn it on for the one noisy client first.

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

An agent supplies `title`, `preface` and `questions`. It never supplies
`project`, `branch` or `diff`: the CLI derives those from the working
directory and overwrites anything a Set claims.

```yaml
title: Rate limiting for the public API   # required
preface: |                                # optional markdown; everything the
  Why this needs deciding, in enough      #   human needs without seeing the
  detail to answer without asking back.   #   agent's session
questions:
  - label: Q1                             # agent-owned and opaque; the Response
    text: Where should the counter live?  #   answers by this name
    options:                              # optional
      - n: 1                              # the number the human selects
        text: In-process, per instance.
        recommended: true                 # the Recommendation; at most one
      - n: 2                              #   per question
        text: In Redis, shared.
    subquestions:                         # optional, and leaves: no third level
      - letter: a                         # named Q1a — parent's label + letter
        text: What should Retry-After say?
        options: [...]                    # same shape as a Question's
```

The rest of the grammar: a Set needs a non-empty title, labels are distinct
across the Set, Option numbers are distinct within a question, and there is no
multi-select — an Answer carries one `selected`.

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

REST lives under `/api/v1/` to stay clear of `/api/{fn_name}`, where the web
UI's Leptos server functions live. Everything not claimed above is the UI's.

## Development

```console
$ cargo test              # unit, schema and end-to-end tests
$ cargo clippy --all-targets
$ cargo fmt
$ cargo leptos build      # both halves of the UI, into target/site
$ nix fmt                 # the Nix files
$ nix flake check         # the NixOS module, in a VM (Linux only)

$ tools/generate-icons.sh # the PWA icons, after editing their SVG
```

`cargo test` covers the round trip in-process. `nix flake check` boots a VM with
the NixOS module enabled and puts a Question Set through it again, for the sake
of everything the module wraps around that round trip: a unit that starts itself
at boot, the state directory systemd hands over, a database that survives the
service being stopped and started under a waiting agent, a server serving the
site it was packaged with rather than a working tree's, and the CLI on `PATH`
with nothing set in the environment. It needs a Linux host to boot the guest on,
so on macOS the check is absent rather than failing.

`assets/` is copied verbatim into the site root by `cargo leptos`: the web
manifest, the icons and the service worker. They cannot live under `/pkg/` with
the wasm and the CSS — a service worker only controls the paths beneath the one
it was served from, so one under `/pkg/` could never show a notification for
`/sets/12`. The worker itself does no caching; every page is rendered against
live SQLite, and a cached copy of a Set that has since been answered is worse
to the human than a failure to load.

The icons are all one SVG, `assets/icons/askance.svg`, rasterized by the script
above (using `resvg` from the dev shell) to the PNG sizes the manifest and iOS
ask for. The PNGs are committed so a build needs nothing but cargo — edit the
SVG and re-run the script rather than touching them.

The tests run the real server in-process, so the round trip they check is the
one an agent gets — including the quickstart above, whose example files
[`crates/cli/tests/ask.rs`](crates/cli/tests/ask.rs) drives end to end, taking
the human's part over the API the page's **Submit** posts through. The UI
compiles natively for that, so `cargo test` covers the server-rendered pages
too.

`askance-frontend` — the wasm half of the UI — is a workspace member but not a
default one: it turns on `leptos/hydrate`, which cannot coexist with the
`leptos/ssr` everything else needs. Only `cargo leptos` builds it.
