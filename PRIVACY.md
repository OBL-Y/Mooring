# Privacy policy

Mooring doesn't collect, send or sell any personal data. It has no analytics, no tracking, no accounts and no servers of its own.

## What stays on your computer

Mooring keeps everything it needs locally, in your user folder:

- **Saved layouts** (`layouts.json`): for each display setup, the position, size and state of your app windows, along with each window's app name, title and window type. Titles can include things like document names or web page titles.
- **Settings** (`settings.json`).
- **Logs**: what Mooring did and when, for troubleshooting.

| | Windows | macOS |
| --- | --- | --- |
| Layouts and settings | `%APPDATA%\io.github.obl-y.mooring` | `~/Library/Application Support/io.github.obl-y.mooring` |
| Logs | `%LOCALAPPDATA%\io.github.obl-y.mooring\logs` | `~/Library/Logs/io.github.obl-y.mooring` |

None of this leaves your computer unless you send it to someone yourself. Uninstalling Mooring and deleting these folders removes it all.

**Export Diagnostics** puts the logs, layouts and settings, plus your OS version and display setup, into a zip in your Downloads folder. It's only shared if you attach it somewhere, for example to a bug report. Because it includes window titles, check it before sharing.

## The one network connection: updates

To check for a new version, Mooring downloads a small file (`latest.json`) from this project's GitHub Releases. It does this when it starts and then once a day. If you click **Install and Restart**, it downloads the new version from GitHub too.

These requests go only to GitHub. Like any website, GitHub sees your IP address and basic request details, under the [GitHub Privacy Statement](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement). Mooring sends nothing about you, your windows or your settings with them.

To stop the automatic checks, turn off **Check for updates automatically** in Mooring's Settings.

## Permissions

- **macOS Accessibility:** used only to read and move other apps' windows.
- **Start at login:** Mooring adds a login item (macOS) or a startup entry (Windows) so it runs when you sign in. You can turn it off in Settings.

## Third parties

Mooring is built with open-source components, including [Tauri](https://tauri.app). None of them send data anywhere. Downloads and updates are hosted by [GitHub](https://github.com), and Windows releases are code-signed through [SignPath](https://signpath.org); signing happens when a release is built and involves no data about users.

## Changes and contact

If this policy changes, the change will be in this file's history and mentioned in the release notes. Questions: open a [discussion](https://github.com/OBL-Y/Mooring/discussions) or email mooringdock@proton.me.
