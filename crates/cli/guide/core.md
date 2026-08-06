# Asking the human

Askance carries a Question Set from a coding agent to the human and blocks
until it comes back answered. The human answers on a phone, away from the
terminal, so a wait of hours is the tool working rather than the tool failing.

This Guide is everything the binary knows about asking well. It ships inside
the binary, so `askance guide` — or `askance` with no arguments — is the whole
of the documentation an agent needs.

## The CLI contract

Verbatim, as shipped:

```
Submit a Question Set and block until the human answers it.

Prints the Response as YAML on stdout and exits 0. Nothing else is ever written to stdout, so the agent can parse it as it stands.

Usage: askance ask [OPTIONS] [FILE]

Arguments:
  [FILE]
          The Question Set, as YAML. Read from stdin when absent

Options:
      --server <SERVER>
          Base URL of the Askance server

          [env: ASKANCE_SERVER=]
          [default: http://127.0.0.1:8422]

  -h, --help
          Print help (see a summary with '-h')
```

Set shape — `title` (required), `preface`, and
`questions[].{label, text, options[].{n, text, recommended},
subquestions[].{letter, text, options}}`.

Response shape — `answers[].{label, selected, free_text, unanswered}` plus a
set-level `comment`.

## Running the ask

**Run `askance ask` as a background shell command** — in Claude Code, a Bash
call with `run_in_background: true`. The call blocks until the human answers,
with no timeout, and that may be hours: the whole point is that they are not at
the terminal. A foreground tool call here hangs the session. The harness wakes
the agent when the Response arrives.

Pipe the Set in on stdin — no file to name, and nothing left behind:

```
askance ask <<'YAML'
title: …
questions:
  - label: Q1
    text: …
YAML
```

Quote the heredoc delimiter (`<<'YAML'`, not `<<YAML`) so the shell leaves the
Set alone — backticks and `$` are ordinary characters in prose and in a diff.

A Set refused as malformed is not the transport breaking: the server is up and
answering, and the fault is in what was sent. Fix the Set and send it again —
the refusal names the Question at fault, and the server is local, so the round
trip costs almost nothing.

While waiting, do any work that does not depend on the answers. Don't speculate
about what the human will say, and don't start work the answers might throw
away.

## Reading the Response

Stdout is the Response YAML and nothing else — all chatter goes to stderr — so
it parses as it stands:

```yaml
answers:
  - label: Q11
    selected: 1
    free_text: Start there, revisit if the catalog case gets stronger.
  - label: Q11a
    unanswered: true
  - label: Q12
    free_text: The nightly export job hammers that endpoint on purpose.
comment: |
  On Q11a I genuinely don't know — pick whatever's least work to change later.
```

That holds for the file a harness collects a background command into, where
the two streams land together: a wait that goes to plan is silent, and the
little the CLI has to say while reconnecting is written as a YAML comment. Hand
the whole thing to a parser.

Every Question and Sub-question in the Set comes back exactly once, so there is
never anything to infer about what the human passed over:

- `selected` → the number of the Option they picked; `selected: 1` on `Q11` is
  `Q11.1`.
- `free_text` → their own words. Alongside `selected` it is the rationale or a
  qualification; on its own it is an answer of their own instead of one of the
  Options, and it wins over the Options offered.
- `unanswered: true` → **still open.** Ask a brief follow-up. Never read it as
  accepting the Recommendation.
- `comment` → about the Set as a whole rather than any one Question. Read it
  before acting on the answers; it may reframe them.

A Response of nothing but `unanswered` entries plus a `comment` is a valid
counter-question. It means the human is not answering as asked — take the
discussion back a step rather than putting the same Set again.
