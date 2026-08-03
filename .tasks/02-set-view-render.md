# 02. Set view: render the ask

## What to build

Clicking a pending Set opens its view: the rendered Preface first, then the
Questions in order, ready to be answered (answer *state* and submit are task
03 — this task is the read-only rendering being complete and correct).

- The Preface is markdown, rendered server-side (pulldown-cmark or similar,
  sanitized) — no JS markdown parser ships to the client.
- Each Question shows its label and text, then its Options as a radio group
  (no multi-select — the schema cannot express it), then a free-text field.
  Sub-questions render the same way, nested one level under their parent,
  labelled `Q7a`-style.
- The Recommendation (★ Option) is visually highlighted but **never
  pre-selected** — nothing is selected on load, so unread recommendations
  cannot be accidentally submitted.
- A set-level comment box sits at the end.
- Questions or Sub-questions with no Options render just text + free-text
  field; the Diff section is task 04.

## Acceptance criteria

- [ ] A Set using every grammar feature — Options, Sub-questions, mixed nodes
      (a Question with both its own Options and Sub-questions), questions
      with no Options — renders correctly and in order
- [ ] The Preface renders as HTML from markdown, server-side
- [ ] ★ Options are visibly marked and no Option is pre-selected on load
- [ ] Every Question and Sub-question has a free-text field; the Set has one
      comment box
