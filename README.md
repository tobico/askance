# Askance

A single-user web service and companion CLI through which coding agents put
their questions to a human and block until answered. An agent submits a
**Question Set**; the human answers it from any device on the tailnet; the
agent's `askance ask` — which has been waiting the whole time — prints the
**Response** and exits 0.

The vocabulary in bold throughout is the project's, and is defined in
[CONTEXT.md](CONTEXT.md).

## Status

The agent-facing contract works end to end: submit, wait, answer, deliver.
The web UI has started landing — the server serves a pending list of the Sets
waiting on the human, and opening one shows the whole ask: its Preface, its
Questions, and the fields to answer them in. Submitting is not wired up yet,
so the quickstart below still plays the human's part with `curl`. See
[the roadmap](docs/roadmaps/v1/ROADMAP.md) for what comes next.

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
on <http://127.0.0.1:8422/>, which currently shows the pending Sets. It creates
`askance.db` in the working directory on first run. Leave it running; check it
in a third terminal if you like:

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

### 4. Answer (terminal 3)

This is the human's part, which the web UI will take over. Post a Response to
the Set the CLI is waiting on — id `1`, from the line it printed in step 3:

```console
$ curl -X POST --data-binary @examples/response.yaml \
    -H 'Content-Type: application/yaml' \
    http://127.0.0.1:8422/api/v1/sets/1/response
set_id: 1
submitted_at: 2026-08-03T05:32:43.784Z
```

[`examples/response.yaml`](examples/response.yaml) answers every Question in
the example Set, leaves one explicitly open, and adds a set-level comment.

### 5. Delivery

Back in terminal 2, the still-waiting CLI has printed the Response and exited:

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

That is the loop. Run step 3 again for Question Set 2, and answer it at
`/api/v1/sets/2/response`.

## Configuration

No app-level auth: the tailnet is the perimeter, so everything defaults to the
loopback interface.

| Variable | Used by | Default | What it is |
| --- | --- | --- | --- |
| `ASKANCE_SERVER` | CLI | `http://127.0.0.1:8422` | Base URL the CLI submits to and waits on. Also `--server`. |
| `ASKANCE_LISTEN` | server | `127.0.0.1:8422` | Address and port to bind. Bind a tailnet address to answer from other devices. Also `--listen`. |
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

$ tools/generate-icons.sh # the PWA icons, after editing their SVG
```

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
one an agent gets — including the quickstart above, which is driven against
these very example files by
[`crates/cli/tests/ask.rs`](crates/cli/tests/ask.rs). The UI compiles natively
for that, so `cargo test` covers the server-rendered pages too.

`askance-frontend` — the wasm half of the UI — is a workspace member but not a
default one: it turns on `leptos/hydrate`, which cannot coexist with the
`leptos/ssr` everything else needs. Only `cargo leptos` builds it.
