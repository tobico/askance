# Releasing

A release is a tag and nothing else.
[`release.yml`](../.github/workflows/release.yml) fires on `v*`: it builds the
viewer once, then one binary per platform on a runner of that platform's own
architecture, runs each binary it built, publishes the four as a GitHub Release
under the tag, and finally commits [`nix/release.json`](../nix/release.json) to
`main` so the flake fetches what was just published. None of that is
hand-driven, and nothing in it is hand-edited afterwards.

A tag with a hyphen in it — `v0.1.0-rc.1` — is semver's own spelling of a
pre-release, and the workflow marks the Release as one. That is the difference
between a tag that ships and one that only rehearses the pipeline: GitHub keeps
a pre-release off `releases/latest`, which is the url the README's install
command asks for.

## Before you tag

- **The version in [`Cargo.toml`](../Cargo.toml) matches the tag without its
  `v`.** Nothing checks this. The manifest takes its version from the tag while
  the binary reports the one it was compiled with, so a mismatch ships a binary
  that disagrees with the flake about what it is — and, where the tag is the
  higher of the two, an Update Notice naming an update that is already
  installed.
- **The commit is already on `main`.** The manifest job checks out `main` rather
  than the tag, so a tag on a branch publishes a Release whose manifest lands on
  a `main` that does not contain the code.
- **CI is green on that commit.** `release.yml` builds each binary and runs it,
  but it runs no tests; those are `ci.yml`'s, and `ci.yml` does not run on tags.

## Tagging

```console
$ git tag -a v0.1.0 -m 'Askance v0.1.0' <sha-on-main>
$ git push origin v0.1.0
```

## After the run

The workflow checks its own manifest: it re-downloads every published asset
through the urls the manifest records and fails if a hash disagrees. What is
left to check by hand is the part no workflow sees — the install story a
newcomer actually follows.

1. **`releases/latest` resolves to the new tag.**

   ```console
   $ curl -sSI -o /dev/null -w '%{http_code}\n' \
       https://github.com/tobico/askance/releases/latest/download/askance-linux-x64
   200
   ```

   A `404` means GitHub still has no release that is not a pre-release, which
   means the tag carried a hyphen.

2. **The README's install command, verbatim**, somewhere `askance` is not
   already on the `PATH`. Then `askance --version`, which prints the tag without
   its `v`.

3. **The flake, refreshed past nix's cache** — after the manifest commit has
   landed on `main`, which is a job later than the Release itself:

   ```console
   $ nix run --refresh github:tobico/askance#askance -- --version
   ```

   What it prints is the manifest's version, and so the tag's.

4. **The manifest on `main`** names the new version and carries all four nix
   systems, committed by `github-actions[bot]` as
   `chore: release manifest for <tag>`. That commit deliberately starts no CI
   run — [the git workflow](agents/git-workflow.md#exception-the-release-manifest)
   records why it is the one write to `main` that skips review.

5. **The Update Notice**, on a server still running the previous version: the
   pending list gains a banner naming the new one, and
   [How to update](../README.md#updating) is where its link lands. The server
   asks GitHub at startup and daily after, so restart the old server rather than
   waiting a day.

## v0.1.0: going live

The first real release. `v0.1.0-rc.1` already exists as a pre-release — it
rehearsed this pipeline and is the version the manifest currently points at —
and it stays exactly where it is: being a pre-release is what keeps it out of
`releases/latest` once there is something better to find there.

**What has to be on `main` first.** The tag is what the release is built from,
so both of these have to have merged before it is pushed:

- **#7, Update Notice.** The published rc predates it: `askance serve --help` on
  that binary lists no `--no-update-check`, and the README documents one.
- **The adoption docs**, stacked on #7. The banner links
  `https://github.com/tobico/askance#updating`, and the `## Updating` section it
  points at arrives with them.

Then tag the `main` commit that has both. `Cargo.toml` says `0.1.0`, so the tag
is `v0.1.0`, and the two sections above are the rest of it.

### What the rehearsal showed

Every install path the README documents, exercised on 2026-08-09 against
`v0.1.0-rc.1` — before either pull request merged:

| Path | Result |
| --- | --- |
| The curl one-liner, run verbatim | **`404`**, curl exiting 22. `releases/latest` does not resolve while the only release is a pre-release, and `api.github.com/…/releases/latest` answers `404` for the same reason. |
| The same asset by tag | `200`. All four assets are published under the names the README lists, and `askance-linux-x64` is a static-pie ELF that prints `askance 0.1.0`. |
| `nix run github:tobico/askance` | The server binds, `/api/v1/health` answers `ok`, and the viewer's page is a `200`. |
| `nix run github:tobico/askance#askance -- --version` | `askance 0.1.0`. |
| The Update Notice's verdict | A server built at `0.1.0`, asking a stand-in releases endpoint that answers `v9.9.9`, reports `{"Available":{"version":"9.9.9"}}` on `/api/ui/update`, and the bundle it then serves carries both the banner's text and its link. |
| The banner's link | `https://github.com/tobico/askance#updating`. GitHub slugs `## Updating` to that anchor, and the section is absent from `main`'s README until the adoption docs merge. |

Two things there are **not** claimed to work yet, and the tag is what settles
both:

- **The curl one-liner.** Nothing is wrong with the command — the by-tag url it
  would resolve to serves the same asset, and does. What is missing is a
  `latest` for it to resolve to, which is the first check under **After the
  run** above.
- **The Update Notice against GitHub itself.** It was exercised against a
  stand-in endpoint because GitHub has nothing newer than the running version to
  report — and will not at v0.1.0 either, since v0.1.0 is then what a server is
  running. Its first real appearance is at the release after this one; what is
  proven now is every piece underneath it.

The banner was not rendered in a browser, because the rehearsal environment has
no headless browser that works. The viewer's own tests
([`web/tests/update.test.tsx`](../web/tests/update.test.tsx)) cover the render,
and the link above was read out of the bundle the running server actually
served.
