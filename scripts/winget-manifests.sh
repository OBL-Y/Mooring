#!/usr/bin/env bash
# Writes the winget manifests for a release, laid out as in microsoft/winget-pkgs.
# Usage: scripts/winget-manifests.sh <tag> [output dir]   e.g. v0.1.0 packaging/out
set -euo pipefail
tag="$1"
out="${2:-packaging/out}"
version="${tag#v}"
# Installers are named after the app version, without a pre-release suffix.
file_version="${version%%-*}"
installer="Mooring_${file_version}_x64-setup.exe"
url="https://github.com/OBL-Y/Mooring/releases/download/${tag}/${installer}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
"$(dirname "$0")/release-file.sh" "$tag" "$installer" "$tmp/$installer"
sha="$(shasum -a 256 "$tmp/$installer" | cut -d' ' -f1 | tr 'a-f' 'A-F')"

dir="$out/manifests/o/OBL-Y/Mooring/$version"
mkdir -p "$dir"
schema="1.12.0"

cat > "$dir/OBL-Y.Mooring.yaml" <<YAML
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.version.$schema.schema.json
PackageIdentifier: OBL-Y.Mooring
PackageVersion: $version
DefaultLocale: en-US
ManifestType: version
ManifestVersion: $schema
YAML

cat > "$dir/OBL-Y.Mooring.installer.yaml" <<YAML
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.installer.$schema.schema.json
PackageIdentifier: OBL-Y.Mooring
PackageVersion: $version
InstallerType: nullsoft
InstallModes:
  - interactive
  - silent
  - silentWithProgress
UpgradeBehavior: install
Installers:
  - Architecture: x64
    InstallerUrl: $url
    InstallerSha256: $sha
ManifestType: installer
ManifestVersion: $schema
YAML

cat > "$dir/OBL-Y.Mooring.locale.en-US.yaml" <<YAML
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.defaultLocale.$schema.schema.json
PackageIdentifier: OBL-Y.Mooring
PackageVersion: $version
PackageLocale: en-US
Publisher: OBL-Y
PublisherUrl: https://github.com/OBL-Y
PublisherSupportUrl: https://github.com/OBL-Y/Mooring/issues
PackageName: Mooring
PackageUrl: https://github.com/OBL-Y/Mooring
License: MIT
LicenseUrl: https://github.com/OBL-Y/Mooring/blob/main/LICENSE
Copyright: Copyright (c) 2026 OBL-Y
ShortDescription: Puts your windows back where they were when you redock your laptop.
Description: Mooring remembers where your windows and desktop icons sit for each display setup, and puts them back when you dock or undock your laptop.
Moniker: mooring
Tags:
  - docking
  - monitors
  - multi-monitor
  - window-manager
  - productivity
ReleaseNotesUrl: https://github.com/OBL-Y/Mooring/releases/tag/$tag
ManifestType: defaultLocale
ManifestVersion: $schema
YAML

echo "Wrote $dir"
