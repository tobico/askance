# 04. Manifest commit-back

## What to build

Once the assets are published, the workflow records what it published in a
manifest and commits it to `main`. That manifest is the whole interface to the
binary flake (stage 04): the flake fetches by url and verifies by hash, and
nothing about it is hand-edited. Upkeep per release has to be zero, or the
flake goes stale the first time someone forgets.

It lives at `nix/release.json`, beside the flake code that will read it, and is
keyed by **nix system name** rather than asset name, because a nix system name
is what the consumer already has in hand. Hashes are **SRI**, which is what
`fetchurl` takes verbatim — no conversion step at the point of use:

```json
{
  "version": "0.1.0",
  "systems": {
    "x86_64-linux": {
      "url": "https://github.com/tobico/askance/releases/download/v0.1.0/askance-linux-x64",
      "hash": "sha256-…"
    }
  }
}
```

Hash the assets as published rather than the local build outputs, so that what
the manifest promises is what an adopter actually downloads.

This job writes to `main` directly, bypassing the pull request process the
project otherwise follows — a deliberate exception, taken because a manifest
that needs a merge per release is a manifest that gets skipped. Note it where
the review process is recorded, so the exception is documented rather than
discovered.

## Acceptance criteria

- [ ] After a release publishes, `main` carries `nix/release.json` naming that
      version, with a url and SRI hash for each of the four nix systems
- [ ] Each hash verifies against the asset downloaded from its own url
- [ ] The commit is attributable to the workflow rather than to a person
- [ ] The push to `main` does not retrigger the release workflow or CI in a loop
- [ ] `docs/agents/git-workflow.md` records that this workflow writes to `main`
      without a PR, and why
