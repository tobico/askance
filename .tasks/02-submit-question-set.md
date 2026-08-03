# 02. Submit a Question Set

## What to build

The Question Set enters the system. Define the Set types in the `schema`
crate with serde + serde-saphyr, encoding the wire format from PLAN.md:
`title`, markdown `preface`, `questions[]` with agent-owned opaque `label`,
`text`, optional `options[]` (`n`, `text`, `recommended`) and optional
`subquestions[]` (`letter`, `text`, `options[]`). Optional `project`,
`branch`, and `diff` fields ride along (the CLI supplies them in task 04;
the server stores them opaquely).

Validation enforces the question grammar's invariants, distinct from mere
deserialization: two levels maximum (Sub-questions are leaves), at most one
`recommended` Option per Question or Sub-question, no multi-select, required
non-empty `title`. Validation lives in `schema` so task 04's CLI reuses it
identically.

`POST /api/v1/sets` accepts the YAML body, validates, persists to SQLite,
stamps `id` and `created_at`, and returns the `id`. Rejections name the
offending Question by its label.

## Acceptance criteria

- [ ] A valid example Set (preface, questions with options, a recommended
      option, sub-questions) is stored and the response carries its `id`
- [ ] A Set nesting three levels deep is rejected with an error naming the
      offending Question
- [ ] A Question with two `recommended` Options is rejected with an error
      naming it
- [ ] A Set with a missing or empty `title` is rejected
- [ ] Multi-line `preface` survives a store round trip byte-for-byte
