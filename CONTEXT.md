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

**Question**:
A single labelled decision put to the human. Carries an agent-supplied opaque
label (e.g. `Q7`), prose text, and optionally Options and Sub-questions.
_Avoid_: item, prompt

**Sub-question**:
A leaf Question nested one level under a Question, labelled by letter
(e.g. `Q7a`). Sub-questions never have their own Sub-questions.
_Avoid_: child question, part

**Option**:
One discrete choice offered on a Question or Sub-question, numbered `.1`,
`.2`, … At most one Option per question is the Recommendation.
_Avoid_: choice, answer option

**Recommendation**:
The Option the agent marks as its preferred answer (the grammar's `★`).
_Avoid_: default, suggestion

**Answer**:
The human's resolution of one Question or Sub-question: a selected Option
and/or free text. Every Question in a Set must have an Answer before the Set
can be submitted.
_Avoid_: reply, response (that's the whole Set)

**Response**:
The submitted collection of Answers for a Question Set, plus an optional
set-level comment. What the waiting agent receives.
_Avoid_: submission, result

**Archive**:
Where Question Sets live after their Response is delivered (or after manual
archiving of an orphaned Set). Permanent, browsable decision history.
_Avoid_: history, log

**Liveness**:
Whether an agent is currently connected and waiting on a Question Set
("agent waiting" vs "agent disconnected"). Display state only — never causes
automatic withdrawal.
_Avoid_: connection status, presence
