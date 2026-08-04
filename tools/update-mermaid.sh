#!/usr/bin/env bash
# Vendor mermaid's single-file build into assets/ at the version pinned below.
#
# The bundle is committed so that `cargo leptos build` needs nothing but cargo,
# the same bargain the generated icons make — but it is never hand-edited:
# change VERSION and run this.
#
# The single-file build rather than the ESM one: mermaid's ESM distribution is
# code-split into dozens of chunks that import each other by relative path, and
# the site root is no place to reassemble a module graph. `dist/mermaid.min.js`
# is the whole of mermaid in one file, and all it does on the way in is set
# `globalThis.mermaid` — which is what assets/diagrams.js then drives.
#
# Needs curl and tar, both of which any system has; nothing here comes from the
# dev shell.
set -euo pipefail

# The pinned version, and the only line to change to move it. What is in
# assets/ is whatever this said when it was last run.
VERSION=11.16.0

cd "$(dirname "$0")/.."

# Unpacked outside the tree: the tarball is 20 MB of sources, maps and types
# around the one file wanted from it.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

curl -sSfL -o "$work/mermaid.tgz" \
  "https://registry.npmjs.org/mermaid/-/mermaid-$VERSION.tgz"
tar -xzf "$work/mermaid.tgz" -C "$work" package/dist/mermaid.min.js

cp "$work/package/dist/mermaid.min.js" assets/mermaid.min.js
echo "assets/mermaid.min.js is now mermaid $VERSION"
