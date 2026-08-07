# Askance

A single-user web service and companion CLI through which coding agents put
their questions to a human and block until answered. Agents submit Question
Sets; the human answers from any device on the tailnet.

## Language

**Question Set**:
A batch of Questions submitted together by one agent, with a Preface and a
title. The unit that appears in the pending list, gets answered, and is
archived.
_Avoid_: request, batch, ticket

**Preface**:
The markdown context that accompanies a Question Set, giving the human
everything needed to understand the Questions without seeing the agent's
session.
_Avoid_: description, context, body

**Postscript**:
Optional markdown the agent closes a Question Set with, rendered in the
section that closes the page, above the set-level comment box it shares that
section with — suggested discussion topics, or whatever else the human might
take up in their comment. Named in the table of contents like the Preface,
and headed for the box alone on a Set that closed without one. Not a
Question: a blank comment beneath it means nothing to add, never Unanswered.
_Avoid_: epilogue, closing, anything-else question

**Question**:
A single labelled decision put to the human. Carries an agent-supplied opaque
label (e.g. `Q7`), markdown prose text, and optionally Options and
Sub-questions.
_Avoid_: item, prompt

**Sub-question**:
A leaf Question nested one level under a Question, labelled by letter
(e.g. `Q7a`). Sub-questions never have their own Sub-questions.
_Avoid_: child question, part

**Heading**:
A Question carrying Sub-questions and no Options of its own. Its text heads
them rather than asking anything: it is drawn without a field, and no Answer
comes back for it — a Response carrying one is refused. Read off the shape,
never declared.
_Avoid_: group, parent question, section

**Option**:
One discrete choice offered on a Question or Sub-question, numbered `.1`,
`.2`, … Its text is a label rather than prose, so markdown in it is inline
markup only. At most one Option per question is the Recommendation.
_Avoid_: choice, answer option

**Recommendation**:
The Option the agent marks as its preferred answer (the grammar's `★`).
_Avoid_: default, suggestion

**Answer Table**:
A question's Options declared with tabular data, one row per Option, drawn
by the viewer as a table whose rows are the selectable Options — the row is
picked in place of a list entry, and the Recommendation and the selection
are marked on the row.
_Avoid_: options table, comparison table (the Guide's generic layout
advice), grid

**Answer**:
The human's resolution of one Question or Sub-question: a selected Option
and/or free text. A Question left without an Answer at submission is
Unanswered.
_Avoid_: reply, response (that's the whole Set)

**Unanswered**:
The explicit state of a Question the human chose not to resolve when
submitting. The agent must treat it as still open — typically because the
set-level comment redirects the discussion.
_Avoid_: skipped, blank

**Response**:
The submitted collection of Answers (and Unanswered markers) for a Question
Set, plus an optional set-level comment. What the waiting agent receives.
May contain zero Answers.
_Avoid_: submission, result

**Diff**:
The uncommitted changes (including untracked files) of the asking repo,
captured by the CLI at send time and attached to every Question Set, so code
approval can happen in the web UI.
_Avoid_: patch, changeset

**Diagram**:
A mermaid fence in a Preface, a Question or a Postscript, rendered visually
in the viewer.
Degrades to its readable source text whenever it cannot render.
_Avoid_: chart, graph, figure

**Guide**:
The agent-facing usage instructions shipped inside the CLI and printed by
it, so an agent needs nothing beyond the binary to learn how to ask. Split
into a core that every ask needs and Topics fetched when their task arises.
_Avoid_: help, manual, docs

**Topic**:
A task-scoped section of the Guide (e.g. gates), split out so an agent pays
its reading cost only when the task is at hand — at which point it is
required reading, never optional.
_Avoid_: section, chapter

**Archive**:
Where Question Sets live after their Response is delivered (or after manual
archiving of an orphaned Set). Permanent, browsable decision history.
_Avoid_: history, log

**Liveness**:
Whether an agent is currently connected and waiting on a Question Set
("agent waiting" vs "agent disconnected"). Display state only — never causes
automatic withdrawal.
_Avoid_: connection status, presence
