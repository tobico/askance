# Asking the human

Askance carries a Question Set from a coding agent to the human and blocks
until it comes back answered. The human answers on a phone, away from the
terminal, so a wait of hours is the tool working rather than the tool failing.

This Guide is everything the binary knows about asking well, and it ships
inside the binary: `askance guide` — or `askance` with no arguments — is where
an agent starts, and the Topics named below are the rest of it. Nothing else
has to be found.

## Required topic guides

The Guide is this core plus its Topics, each printed by `askance guide <topic>`.
A Topic is part of the Guide rather than an extra. It is split out only so its
reading cost falls when the task that needs it arrives, instead of before every
ask.

- **`gates`** — before authoring a confirmation gate, meaning any Set that asks
  the human for approval to proceed, to commit or to ship, the agent **MUST**
  run `askance guide gates` first and write the Set the way it says. A gate is
  written differently from an ordinary Set and its Response is read far more
  strictly; that strictness is the Topic's whole subject, and getting it wrong
  lands work nobody approved.

## Question labels

Label every Question, so each Answer keys unambiguously back to what was asked.
Not every Question needs Options — a bare clarifying Question just carries a
label. Add Options where there are discrete choices, and then recommend one.

- **Questions** are numbered `Q1`, `Q2`, … **monotonically across the whole
  session** — never reset the counter for a new Set, so `Q4` still means the
  same Question when either side points back to it later.
- **Sub-questions** append a letter: `Q7a`, `Q7b`. Use them when one Question
  has distinct parts that each need their own Answer.
- **Options** append `.N`: a Question's are `Q7.1`, `Q7.2`, a Sub-question's
  are `Q7a.1`, `Q7a.2`. The **Recommendation** is the `★` appended to an
  Option's number — `Q7.1★` — and there is at most one per Question or
  Sub-question.
- A Question **may carry both its own Options and Sub-questions.** Nothing is
  ambiguous, because an Answer at the Question level is a bare number and one at
  the Sub-question level always carries the letter.
- **Two levels maximum.** A Sub-question is a leaf: it carries Options, it never
  branches further. A decision that feels deeper than that splits into separate
  top-level Questions.

A Question carrying both:

```
Q11 — Which framing should the rewritten README take?
      Q11.1★  Personal curated suite
      Q11.2   Distribution-first catalog
      Q11a — Keep the documented install command?
             Q11a.1★  Keep it
             Q11a.2   Change it
      Q11b — Keep the workflow walkthrough section?
             Q11b.1★  Keep it, updated
             Q11b.2   Drop it
```

## Pacing

Pace by complexity, not by Question count. The unit is the Set: one delivery of
Questions and the Answers that come back. Each Set should carry about one
sitting's worth of decision effort, where the effort is what an Answer costs the
human — not what it costs the agent to ask.

| Question complexity | Per Set |
|---|---|
| **complex** — weighs trade-offs, is open-ended, or has downstream consequences | ~1 |
| **medium** | ~4 |
| **simple** | ~8 |
| **trivial** — a fact, a name, a yes/no, or accepting an obvious default | ~15 |

A Set is a sitting rather than a passing remark: the human sees the whole of
it at once, in a UI built for reading it, where one more cheap Question costs
a tap. The round trip may be hours, so a Set is worth filling. The ceiling on
hard Questions doesn't move, though — thinking effort doesn't parallelize.

Mix freely, one medium alongside a couple of trivial, as long as the total stays
inside the budget. Batch independent Questions right up to it; the labels above
are what keep a full Set unambiguous.

Keep Questions sequential only where a later one genuinely depends on an earlier
Answer — never ask a Question whose very wording would change with an Answer
requested in the same Set. Nothing can be asked mid-Set, so a dependent Question
waits for the next one and costs a whole round trip. Where the dependency is
shallow, enumerate instead of deferring: fold the branches into Options, or hang
them off the parent as Sub-questions.

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

## Authoring the Set

**If the Set asks for approval to proceed — to commit, to land, to ship — stop
and run `askance guide gates` first.** A gate is written differently from
everything below and its Response is read far more strictly, and the Topic is
where both of those live.

One Set is one round, budgeted as above. Because the round trip is expensive,
sweep ahead: carry the Questions that would otherwise wait for the next round or
two, provided none of them depends on an Answer in this one. A trailing open
Question — anything worth knowing before the work starts — costs the human
almost nothing and often saves a whole round trip.

Decide the Questions first, then serialize them:

```yaml
title: Rate limiting for the public API
preface: |
  `POST /v1/messages` has no rate limit, and last night one client sent 40k
  requests in a minute. A limiter can land today, but where the counter lives
  is a product call rather than a technical one.
questions:
  - label: Q11
    text: Which framing should the rewritten README take?
    options:
      - n: 1
        text: Personal curated suite
        recommended: true
      - n: 2
        text: Distribution-first catalog
    subquestions:
      - letter: a
        text: Keep the documented install command?
        options:
          - n: 1
            text: Keep it
            recommended: true
          - n: 2
            text: Change it
  - label: Q12
    text: Anything worth knowing before this starts?
```

Mapping from the labels:

- `label` is the `Qn` label, straight from the session counter — the server
  never assigns one, so the `Q11` here is the `Q11` either side can point back
  to.
- `letter` is the Sub-question suffix; `Q11` plus `a` is answered as `Q11a`.
  Sub-questions are leaves, as above — a third level is refused.
- `recommended: true` is the `★`. At most one per Question or Sub-question.
- Options are optional. A Question with none is a bare clarifying Question, and
  the Answer is whatever the human writes.

Three things to get right:

- **`preface` is not optional in practice.** The human answers without seeing
  the session, so the context that would otherwise sit in the session has to
  live here instead. Markdown. Enough that the Questions make sense cold.
- **Never supply `project`, `branch` or `diff`.** The CLI derives all three from
  the working directory and overwrites whatever the Set claims — including the
  uncommitted Diff, so the human can see what has already been written.
- **Prose does not survive plain YAML scalars.** A colon-space anywhere in a
  Question, an Option or the Preface ends the scalar and the server refuses the
  whole Set — and quoting a command or a log line is exactly when it bites. Use
  a block scalar (`|`), or a folded one (`>-`), for anything longer than a few
  plain words. Markdown inside a block scalar needs no escaping at all, which
  is the other reason to reach for one.

And write it to be **grasped at a glance.** A Set is read on a phone, often
between other things, and one that has to be studied is one that gets put off.
That is a property of the writing rather than of the Questions:

- **Lead the `preface` with the bottom line** — the decision, the state of play,
  the one thing worth having from a single sentence. The context that justifies
  it comes after, for when one sentence isn't enough.
- **Keep the `preface` short.** Context that only one Question needs belongs in
  that Question's `text`, not up front. Aim for Questions that can be answered
  without reading the Preface at all.
- **Prefer a Diagram to prose for structure.** Relationships, flows and state
  are quicker to see than to read, and a ```` ```mermaid ```` fence in the
  `preface` or a Question's `text` is drawn as a Diagram in the viewer. Keep it
  small — roughly ten nodes, so it stays legible on a phone. A fence degrades
  to the source text it was written as wherever it can't be drawn, so it is
  safe to send even to an older viewer.
- **Prefer a comparison table where Options trade off on several axes** — one
  row per Option, one column per axis, rather than a paragraph each.
- **Bold the load-bearing phrases** so skimming lands on them.

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

There is no health probe — the attempt is the probe. If the ask fails for a
reason that isn't the Set — the server down, the connection refused, any other
non-zero exit — **report the failure to the human and wait for instructions.**
There is nowhere else to put the Questions. Say what failed and what is now
waiting on them, then stop: answering on their behalf, or taking the
Recommendations and carrying on, decides in their place the very thing that was
worth asking about.

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
