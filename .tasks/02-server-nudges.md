# 02. The server Nudges

## What to build

The server broadcasts a Nudge — contentless, per the glossary — whenever one
of the three durable changes happens: a Question Set is created, a Response is
submitted, or a Set is archived. A new viewer-facing SSE endpoint (alongside
the existing `/api/ui/` routes) streams those Nudges to any subscriber, with
periodic keep-alives so intermediaries never time the stream out.

Liveness transitions do not Nudge: the waiting/disconnected verdict cycles
with the agent's long-poll rather than changing at clean moments, and the
badge stays with the poll (ADR-0005).

## Acceptance criteria

- [ ] A subscriber to the stream receives a Nudge for each of the three
      durable changes, exercised end to end through the HTTP API
- [ ] An agent's wait opening or closing produces no Nudge
- [ ] The stream carries keep-alives
- [ ] Multiple simultaneous subscribers each receive every Nudge
