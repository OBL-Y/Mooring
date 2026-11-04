# Security policy

## Reporting a problem

Please **don't** report security problems in a public issue.

Use GitHub's private reporting instead: on the repo's **Security** tab, click
**Report a vulnerability**. Only the maintainers can see the report. If that
isn't available to you, email **mooringdock@proton.me**.

Include what you found, how to reproduce it, and which version and OS you used.
You should hear back within a week. Once a fix is released, we're happy to
credit you in the release notes if you'd like.

## Supported versions

Only the latest release gets security fixes. Mooring updates itself, so most
people are on it already.

## What counts

Mooring moves other apps' windows (with Accessibility access on macOS), starts
at login, and updates itself from GitHub Releases. Problems in any of these are
in scope, for example:

- a way to make Mooring install an update that isn't signed with the project's key,
- a way for another app or a crafted file (such as `layouts.json` or
  `settings.json`) to make Mooring run code or do something it shouldn't,
- anything in the installers or the release process that could let someone
  ship a modified Mooring.

Unsigned installers (SmartScreen and Gatekeeper warnings) are a known gap, not a
vulnerability; the README's install steps explain them.
