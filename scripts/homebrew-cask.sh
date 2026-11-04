#!/usr/bin/env bash
# Writes the Homebrew cask for a release, for the OBL-Y/homebrew-mooring tap.
# Usage: scripts/homebrew-cask.sh <tag> [tap dir]   e.g. v0.1.0 ../homebrew-mooring
set -euo pipefail
tag="$1"
tap="${2:-packaging/out/homebrew-mooring}"
version="${tag#v}"
# The DMG is named after the app version, without a pre-release suffix.
file_version="${version%%-*}"
dmg="Mooring_${file_version}_universal.dmg"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
"$(dirname "$0")/release-file.sh" "$tag" "$dmg" "$tmp/$dmg"
sha="$(shasum -a 256 "$tmp/$dmg" | cut -d' ' -f1)"

mkdir -p "$tap/Casks"
cat > "$tap/Casks/mooring.rb" <<RUBY
cask "mooring" do
  version "$version"
  sha256 "$sha"

  url "https://github.com/OBL-Y/Mooring/releases/download/v#{version}/Mooring_#{version.sub(/-.*/, "")}_universal.dmg"
  name "Mooring"
  desc "Puts your windows back where they were when you redock your laptop"
  homepage "https://github.com/OBL-Y/Mooring"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates true
  depends_on macos: :ventura

  app "Mooring.app"

  uninstall launchctl: "Mooring",
            quit:      "io.github.obl-y.mooring",
            delete:    "~/Library/LaunchAgents/Mooring.plist"

  zap trash: [
    "~/Library/Application Support/io.github.obl-y.mooring",
    "~/Library/Caches/io.github.obl-y.mooring",
    "~/Library/Logs/io.github.obl-y.mooring",
    "~/Library/WebKit/io.github.obl-y.mooring",
  ]

  caveats <<~EOS
    Mooring isn't notarized by Apple yet. The first time you open it, go to
    System Settings › Privacy & Security and click Open Anyway, then allow
    Mooring under Accessibility so it can move windows.
  EOS
end
RUBY

echo "Wrote $tap/Casks/mooring.rb"
