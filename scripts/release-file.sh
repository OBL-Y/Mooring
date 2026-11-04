#!/usr/bin/env bash
# Downloads one file from a Mooring GitHub release.
# Usage: scripts/release-file.sh <tag> <file name> <output path>
set -euo pipefail
tag="$1" name="$2" out="$3"
if command -v gh > /dev/null; then
  gh release download "$tag" --repo OBL-Y/Mooring --pattern "$name" --output "$out" --clobber
else
  curl -fsSL -o "$out" "https://github.com/OBL-Y/Mooring/releases/download/$tag/$name"
fi
