# 03. The gates Topic

## What to build

The Topic machinery, with `gates` as its first and only member. `askance
guide gates` prints the gates Topic; a topic that does not exist is an
error on stderr, non-zero exit, listing the Topics that do; `askance guide`
stays the core Guide.

The gates content moves from the canonical grammar's "Confirmation gates"
section at its current depth: a gate is a one-question Set whose Preface
does all the work; a structure Diagram is the default and prose needs the
excuse; the delta rules (diagram the delta, not the system; tag nodes
`new`/`modified`/`removed`; a before/after pair only when structure is
reshaped; a sequence diagram for a new runtime flow); and the strict reading
of the Response — a selected proceed Option is approval, `unanswered` is
not, a counter-question reopens the discussion, anything ambiguous stays
shut, fail closed. Genericized: "a commit-approval gate", no skill names.

Core's point-of-relevance triggers land in the same task: the authoring
section's mention of approval asks sends the agent to `askance guide gates`
before writing one, echoing the contract section from task 02.

## Acceptance criteria

- [ ] `askance guide gates` prints the gates Topic and exits 0
- [ ] `askance guide nonsense` exits non-zero with an error naming the
      Topics that exist, and stdout stays clean
- [ ] The gates Topic carries the full gates guidance — degenerate Set,
      delta Diagram rules, strict reading, fail closed — with no skill
      names
- [ ] The core Guide points at `gates` both in the contract section and at
      the point of authoring relevance
