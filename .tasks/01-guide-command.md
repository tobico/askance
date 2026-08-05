# 01. The Guide takes the stage

## What to build

The Guide reaches an agent through the binary alone: `askance guide` prints
the core Guide as markdown on stdout and exits 0, and bare `askance` (no
arguments) does the same instead of clap's usage error. The top-level
`--help` about-text points at the Guide so an agent that starts there finds
it.

The Guide's source is a markdown file in this repo, embedded with
`include_str!` — never generated at runtime, so what ships is what was
reviewed. Its first content is the sections nearest the binary, adapted from
tobico-skills' canonical QUESTION-GRAMMAR.md (transport section): the CLI
contract, running the ask (background command per ADR-0001, heredoc stdin,
pre-parsing bigger Sets), and reading the Response (`selected` / `free_text`
/ `unanswered` / `comment` semantics, the all-unanswered counter-question).
Adaptation rules that apply to all Guide content: the human is "the human",
never "I"; no tobico-skills skill names; Claude Code stays as a named
example of a harness; no chat fallback anywhere.

A test keeps the Guide honest: the CLI-contract block the Guide quotes must
match the real clap `--help` output, rendered by the same binary, so the
document can never lie about the tool it ships in.

## Acceptance criteria

- [ ] `askance guide` prints the core Guide on stdout and exits 0
- [ ] Bare `askance` prints the same Guide and exits 0; `askance --help`
      still prints clap help, whose about-text names the Guide
- [ ] The Guide's quoted CLI contract is asserted equal to the real
      `askance ask --help` output by a test that fails on drift
- [ ] The Guide covers the CLI contract, running the ask, and reading the
      Response, following the adaptation rules above
