# 02. Example skills

## What to build

Two example skills as real files under `examples/skills/`, ready for the README
to quote in task 03. They are what an adopter pastes into their agent's
instructions to get the two workflows Askance is built around.

- **A grilling skill** — the agent stress-tests a plan or design with the human
  before building, putting its questions through `askance ask`.
- **An acceptance-gate skill** — the agent stops before committing or shipping
  and asks for approval. This one **must follow `askance guide gates`**: a gate
  is authored differently from an ordinary Set and its Response is read far more
  strictly. Read that Topic before writing the skill, and let it drive the
  content rather than paraphrasing from memory.

Both are **tool-agnostic markdown** — plain prose and headings that read as
instructions to any CLI coding agent. Deliberately *not* Claude Code's
`SKILL.md` frontmatter format: the target adopter may be running opencode or
something else, and a Claude-specific file would read as not-for-them.

Keep each short enough that quoting it in the README is reasonable — the README
excerpt is the point of contact, and a skill that needs scrolling past in the
shop window is too long.

`examples/` already holds `questions.yaml` and `response.yaml`, which are
wire-format samples rather than agent instructions. The skills go in a
`skills/` subdirectory so the two kinds of example do not sit intermixed.

## Acceptance criteria

- [ ] `examples/skills/` holds a grilling skill and an acceptance-gate skill,
      each a standalone markdown file
- [ ] Neither carries Claude-specific frontmatter or names a specific agent
      product in its instructions
- [ ] The acceptance-gate skill matches what `askance guide gates` actually
      says — checked against the Topic's text, not from memory
- [ ] Each skill is short enough to quote in full in the README without
      dominating the page
- [ ] Each reads as something that works when pasted into an agent's
      instructions verbatim, with no editing required beyond taste
