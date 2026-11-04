# Package managers

Mooring is published to **winget** (Windows) and to its own **Homebrew tap**
(macOS). Both pull the installers from a GitHub release, so they need the repo
to be public and a published (not pre-release) version.

After the one-time setup below, the **Publish to package managers** workflow
(`.github/workflows/publish.yml`) sends every new release to both on its own.
Each half does nothing until its token secret is set.

## winget

Package ID: `OBL-Y.Mooring`. People install it with:

```
winget install OBL-Y.Mooring
```

**First version (by hand).** winget's automation can only update a package
that's already in [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs),
so the first one is a pull request:

1. Publish the release, e.g. `v0.1.0`.
2. Write the manifests: `scripts/winget-manifests.sh v0.1.0`. They land in
   `packaging/out/manifests/o/OBL-Y/Mooring/0.1.0/`.
3. Optional, on Windows: `winget validate packaging\out\manifests\o\OBL-Y\Mooring\0.1.0`
   and `winget install --manifest packaging\out\manifests\o\OBL-Y\Mooring\0.1.0`.
4. Fork `microsoft/winget-pkgs` to the OBL-Y account, copy the `manifests`
   folder into the fork, and open a pull request. Microsoft's checks and a
   reviewer handle it from there; it usually takes a day or two.

**Later versions (automatic).** Once the first version is merged:

1. Create a classic personal access token with the `public_repo` scope.
2. Add it to this repo as the secret `WINGET_TOKEN`.

From then on, publishing a release opens the winget pull request for you, from
the OBL-Y fork.

## Homebrew

Tap: [OBL-Y/homebrew-mooring](https://github.com/OBL-Y/homebrew-mooring).
People install it with:

```
brew install --cask obl-y/mooring/mooring
```

(If Homebrew says the tap isn't trusted, run `brew trust obl-y/mooring` first.)

Homebrew's main cask collection only accepts apps notarized by Apple, which
needs a paid Apple Developer ID; hence our own tap. The cask tells people
about the one-time **Open Anyway** step.

**Set up the tap (once):**

1. Create a public repo `OBL-Y/homebrew-mooring` with a short README.
2. Write the cask into it: `scripts/homebrew-cask.sh v0.1.0 ../homebrew-mooring`,
   then commit and push.
3. Create a fine-grained personal access token with **Contents: read and
   write** on `OBL-Y/homebrew-mooring` only.
4. Add it to this repo as the secret `HOMEBREW_TAP_TOKEN`.

From then on, publishing a release updates the cask's version and checksum.
The cask has `auto_updates true`, because Mooring updates itself; `brew upgrade`
leaves it to the app.

## Trying a release by hand

Run the workflow from the **Actions** tab (**Publish to package managers ›
Run workflow**) with a tag, for example to resend a release after setting up a
token.
