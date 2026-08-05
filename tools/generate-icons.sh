#!/usr/bin/env bash
# Rasterize assets/icons/askance.svg into the PNG sizes the manifest and iOS ask
# for.
#
# The PNGs are committed so that a build needs nothing but node and cargo, but
# they are never hand-edited: change the SVG and run this. The script lives
# outside assets/ because everything in there is copied into the served site
# root, and a build script is not part of the site.
#
# resvg comes from the dev shell, so run this under `nix develop` — or as
# `nix develop --command tools/generate-icons.sh`.
set -euo pipefail
cd "$(dirname "$0")/../assets/icons"

resvg --width 192 --height 192 askance.svg icon-192.png
resvg --width 512 --height 512 askance.svg icon-512.png
# iOS ignores the manifest's icons and takes this one, at this exact size.
resvg --width 180 --height 180 askance.svg apple-touch-icon.png
