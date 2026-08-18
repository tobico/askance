# Asking the human

Askance carries a Question Set from a coding agent to a human, who answers on
a phone, away from the terminal, and it blocks until the answers come back. A
wait of hours is the tool working, not failing.

`askance guide` — or bare `askance` — prints this guide. Topics are printed by
`askance guide <topic>`.

## Required topic guides

- **`gates`** — before authoring a confirmation gate (any Set asking approval
  to proceed, commit, or ship) you **MUST** run `askance guide gates` and
  write the Set the way it says. A gate is authored and read differently, and
  getting it wrong lands work nobody approved.

## Question labels

- Number Questions `Q1`, `Q2`, … monotonically across the whole session.
  Never reset for a new Set, so a label always points back to one Question.
- Sub-questions append a letter — `Q7a` — and serialize under the parent's
  `subquestions` with a bare `letter: a`. They are leaves: two levels maximum,
  never nested further.
- Options number `.1`, `.2`, … Mark at most one per Question or Sub-question
  `recommended: true`.
- A Question may carry Options, Sub-questions, or both. One with Sub-questions
  and no Options is a Heading: its text frames the group, it takes no Answer,
  and a Response that answers it is refused.
- A Question with neither is a bare clarifying question, answered in free
  text.

## Pacing

The unit is the Set: one delivery of Questions and the Answers back. Budget
each Set as roughly one sitting of decision effort — effort measured by what
answering costs the human:

| Question complexity | Per Set |
|---|---|
| **complex** — trade-offs, open-ended, downstream consequences | ~1 |
| **medium** | ~4 |
| **simple** | ~8 |
| **trivial** — a fact, a name, a yes/no, an obvious default | ~15 |

Mix freely within the budget. The round trip may be hours, so fill the Set:
batch every independent Question, and sweep in ones that would otherwise wait
a round or two. Never include a Question that depends on an Answer requested
in the same Set — fold shallow dependencies into Options or Sub-questions;
deep ones wait for the next Set.

## Authoring the Set

**If the Set asks approval to proceed — to commit, to land, to ship — stop
and run `askance guide gates` first.**

Set fields: `title` (required), `preface`, `postscript`, and
`questions[].{label, text, columns, options[].{n, text, recommended, cells},
subquestions[].{letter, text, columns, options}}`. Never supply `project`,
`branch`, or `diff` — the CLI derives them from the working directory,
including the uncommitted diff, so the human sees what has been written.

```yaml
title: Rate limiting for the public API
preface: |
  **A limiter can land today; the open call is where the counter lives.**
  Last night one client sent 40k requests a minute at `POST /v1/messages`.
questions:
  - label: Q11
    text: Where should the counter live?
    columns: [Accuracy, Ops cost]
    options:
      - n: 1
        text: In-process counter
        cells: [Per-node, Nothing to run]
        recommended: true
      - n: 2
        text: Shared Redis counter
        cells: [Exact, A service to run]
    subquestions:
      - letter: a
        text: Send Retry-After on a limited response?
        options:
          - n: 1
            text: Yes
            recommended: true
          - n: 2
            text: Bare 429
  - label: Q12
    text: |
      **Rollout.** Neither part depends on Q11.
    subquestions:
      - letter: a
        text: Behind a feature flag?
        options:
          - n: 1
            text: Flagged
            recommended: true
          - n: 2
            text: Straight on
      - letter: b
        text: Announce in the changelog?
        options:
          - n: 1
            text: Announce
          - n: 2
            text: Silent
postscript: |
  Anything else worth a word — traffic patterns this misses, clients to
  grandfather.
```

`Q12` is a Heading: Options on its Sub-questions, none of its own, so the
Response carries `Q12a` and `Q12b` and no `Q12`.

- **`preface`** — required in practice: the human answers without seeing the
  session. Lead with the bottom line, keep it short, and put context only one
  Question needs in that Question's `text`.
- **`postscript`** — open-ended invitations only, taken up in the set-level
  comment box drawn beneath it. A decision, however small, is a Question
  instead: "Write an ADR for this?" is two Options, priced trivial. And never
  ask "anything else?" as a Question — the comment box already asks it on
  every Set.
- **`columns`/`cells`** — declare a comparison table where Options trade off
  along axes: `columns` names the axes, each Option's `cells` fills them in
  order, and the viewer draws the Options as selectable rows with `text` as
  the leading cell. Never write a markdown table the Options then restate.
- A ```` ```mermaid ```` fence in `preface` or `text` renders as a diagram.
  Prefer one to prose for structure and flows; keep it around ten nodes so it
  stays legible on a phone.
- **Bold the load-bearing phrases** — the Set is skimmed on a phone.
- Use a block scalar (`|` or `>-`) for anything beyond a few plain words: a
  colon-space in a plain scalar ends it and the server refuses the Set.
  Markdown inside a block scalar needs no escaping.

## Running the ask

**Run `askance ask` as a background shell command** — in Claude Code, a Bash
call with `run_in_background: true` — and pipe the Set in on stdin. The call
blocks with no timeout until the Response arrives; a foreground tool call
hangs the session.

```
askance ask <<'YAML'
title: …
questions:
  - label: Q1
    text: …
YAML
```

Quote the heredoc delimiter (`<<'YAML'`) so backticks and `$` pass through
untouched.

- A Set refused as malformed names the Question at fault. Fix the Set and
  resend — the server is local and the retry costs nothing.
- Any other failure — server down, connection refused — means report it to
  the human and stop. Proceeding on your own recommendations decides in their
  place the very thing worth asking about.
- While waiting, do only work no pending Answer could throw away.

## Reading the Response

Stdout is the Response YAML and nothing else — chatter goes to stderr — so
parse it as it stands. That includes the file a background command collects
both streams into: anything the CLI says while waiting arrives as YAML
comments.

```yaml
answers:
  - label: Q11
    selected: 1
    free_text: Start there; revisit if load grows.
  - label: Q11a
    unanswered: true
  - label: Q12a
    selected: 1
  - label: Q12b
    selected: 2
comment: |
  On Q11a, pick whatever is least work to change later.
```

Every Question and Sub-question asked comes back exactly once; a Heading never
does.

- `selected` — the number of the Option picked.
- `free_text` — the human's own words. With `selected`, a rationale; alone, an
  answer of their own that wins over the Options offered.
- `unanswered: true` — still open. Follow up briefly; never read it as
  accepting the Recommendation.
- `comment` — about the Set as a whole, and the reply to the postscript. Read
  it before acting on the Answers; it may reframe them. An absent comment
  means nothing to add, never a question left open.

A Response of nothing but `unanswered` entries plus a `comment` is a
counter-question: take the discussion back a step rather than resending the
same Set.
