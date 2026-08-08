# 03. Release on tag

## What to build

The four artifacts from the previous task become a GitHub Release, published
under the tag that triggered the run. The assets keep their friendly names —
`askance-linux-x64` rather than a target triple — because those names appear
verbatim in the install command the README will document, and a target triple
is not something an adopter should have to decode.

The Release is created only once every leg has succeeded, so a run where one
architecture fails to build publishes nothing rather than a partial set that
looks complete.

A tag matching a pre-release version should produce a Release marked as a
pre-release, so that the throwaway tag in task 05 cannot be mistaken for the
project's first real version — and so `releases/latest` keeps pointing at the
newest stable release.

## Acceptance criteria

- [ ] Pushing a `v*` tag publishes a GitHub Release for that tag carrying
      exactly the four assets, named `askance-linux-x64`,
      `askance-linux-arm64`, `askance-macos-x64` and `askance-macos-arm64`
- [ ] Downloading an asset by its unauthenticated release URL yields a working
      binary — the repo is public, so no token is involved
- [ ] A failing build leg publishes no Release at all
- [ ] A pre-release tag is marked as a pre-release; a plain `vX.Y.Z` tag is not
- [ ] The workflow's write access is scoped to what publishing needs, rather
      than granted to the job that builds
