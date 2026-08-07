# 03. Malformed declarations refuse the Set

## What to build

A declaration that does not add up is an authoring bug, and the Set is
refused at submit — before the human ever sees it — with a message naming
the question at fault, the way other malformed Sets are refused today. The
CLI surfaces the refusal unchanged, and the agent fixes and resends; the
local round trip is nearly free.

Refused shapes:

- An Option whose `cells` count differs from the question's `columns` count.
- `cells` on any Option of a question that declares no `columns`.
- A question declaring `columns` where some Options carry `cells` and
  others do not.

An edge to pin down while building: an empty `columns` list declares a
table with no axes — decide whether that is refused as meaningless or read
as no declaration at all, and say which in the refusal or the docs of the
validation. Whichever way, it must not produce a broken table.

## Acceptance criteria

- [ ] Each refused shape above comes back as a submit-time refusal naming
      the question's label; a well-formed declaration passes
- [ ] A Set with no `columns` anywhere is wholly unaffected by the new
      validation
- [ ] The empty-`columns` edge has one decided, tested behavior
