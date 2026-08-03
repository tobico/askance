# 05. End-to-end example + quickstart

## What to build

Proof the whole contract works from a fresh checkout, and the document that
teaches it. An `examples/` directory with a realistic sample Question Set
(markdown preface, labelled questions, options with a recommendation,
sub-questions), and a README quickstart walking the full loop: enter the dev
shell, start the server, run `askance ask examples/...` in a second
terminal, submit a Response with `curl`, and watch the CLI print it and
exit 0. Include a valid example Response body for the curl step so the
reader doesn't have to compose one.

## Acceptance criteria

- [ ] The sample Set exercises the grammar: preface, options, one
      Recommendation, at least one Sub-question
- [ ] Following the README verbatim on a fresh checkout reproduces the
      ask → answer → deliver loop
- [ ] The README documents the server URL env var and the Set/Response YAML
      shapes (or links where they're defined)
