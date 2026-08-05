# 05. The switch-over

## What to build

The outside world starts running on the Guide alone. This task works mostly
in **~/src/tobico-skills** (its own feature branch, same `<feature>`
naming; commit there, and leave the branch ready to land with that repo's
finish sequence alongside this one), plus one edit outside any repo.

**The tobico-skills sweep.** Delete the canonical QUESTION-GRAMMAR.md at the
repo root, every per-skill copy of it, and `bin/question-grammar.sh`. Then
sweep every SKILL.md plus README.md and VENDOR.md so that no file mentions
askance, the question grammar, a transport, or pacing — question-asking
instructions become plain "ask the user and wait for their answers"
phrasing. The proof the user asked for is that the skills carry nothing:
the global CLAUDE.md line alone routes their questions through askance.

**The global CLAUDE.md collapse.** In ~/.claude/CLAUDE.md, the "Asking
questions" section — including its pacing paragraph — becomes exactly this
single line:

> Never use the AskUserQuestion tool. Put all questions and approvals to me
> through askance: run `askance` once per session for the guide and follow
> it, including the topic guides it requires.

There is no "if askance is not installed" clause: the chat fallback feature
was removed on purpose (grilling Q15).

The live validation — a real grilling session driven by the CLAUDE.md line
alone, through the phone — happens after both branches land and the
deployed binary carries the Guide; it is not a criterion of this task.

## Acceptance criteria

- [ ] In tobico-skills: the canonical grammar, all per-skill copies, and
      the sync script are gone
- [ ] `grep -ri askance` in tobico-skills returns nothing, and no SKILL.md,
      README.md, or VENDOR.md mentions the grammar, a transport, or pacing
- [ ] The skills still read coherently — each question-asking instruction
      stands on its own without the deleted references
- [ ] ~/.claude/CLAUDE.md's "Asking questions" section is exactly the
      single approved line
- [ ] The tobico-skills feature branch is committed and ready to land with
      its repo's finish sequence
