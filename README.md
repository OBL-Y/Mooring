# Mooring

A boat moors at a dock and returns to the same spot, which is what your windows do when you redock.

Mooring is a free, open-source app for **Windows and macOS** that puts your windows back where they were when you plug your laptop into a dock or unplug it. On Windows it also puts desktop icons back in the order you left them.

> **Status: early development (v0.1, not yet released).** The app builds, but it hasn't been tested on real docking setups yet. See the [Roadmap](#roadmap) for what's left before a first release. Help is very welcome; see [Contributing](#contributing).

## The problem

When you undock a laptop, the OS piles every window onto the laptop screen. When you dock again, they stay there, and you drag them back by hand. Windows 11 tries to remember positions, but it doesn't always work. macOS doesn't try at all.

## What Mooring does

- Runs in the system tray (Windows) or menu bar (macOS).
- Every few seconds, saves the position, size and state (normal, maximized or minimized) of every app window, filed under the current set of screens. Each display setup gets its own saved layout, such as "desk with two monitors", "laptop only" or "meeting room TV".
- When screens are added or removed, it stops saving, waits for them to settle, and moves every window back to where it was the last time you used that setup.
- On Windows, it also saves and restores desktop icon positions.
- Lets you **lock** a layout so the automatic saving won't change it.
- Installs with a normal installer. On Windows you can install just for yourself (no admin rights) or for everyone (admin). It adds a Start menu entry and a desktop shortcut, and runs at startup. Startup can be turned off at any time.

## Roadmap

These are the steps from here to a first public release and beyond. Tick them off in PRs as they're done.

### 1. Test on real hardware (needed for v0.1.0)

The code compiles on both systems, but none of this has been tried on a real dock yet.

- [ ] **macOS:** grant Accessibility access, dock and undock, and check that windows go back to the right screens.
- [ ] **macOS:** check minimized windows and windows on other Spaces.
- [ ] **Windows 11:** dock and undock with one and with two external monitors.
- [ ] **Windows 10:** same as above.
- [ ] **Windows:** maximized windows move to the right monitor and stay maximized.
- [ ] **Windows:** monitors with different scaling (e.g. 150% laptop with a 100% monitor) still get exact positions.
- [ ] **Windows:** desktop icons restore (with "Auto arrange icons" off).
- [ ] **Windows installer:** "Only for me" installs without an admin prompt.
- [ ] **Windows installer:** "Everyone" asks for admin and installs to Program Files.
- [ ] **Windows installer:** the Start menu entry and the desktop shortcut are created.
- [ ] **Windows installer:** uninstall removes the app and the startup entry.
- [ ] **Run at startup / Open at login:** on by default after first run, and turning it off in Settings or the tray menu works.
- [ ] Sleep, wake and lid close while docked don't scramble the saved layouts.

### 2. Make it trustworthy to install (needed for v0.1.0)

- [ ] **macOS signing:** sign and notarize the Mac build with an Apple Developer ID. Without it, Gatekeeper blocks the app. The release workflow is already wired for the secrets listed under [Releasing](#releasing).
- [ ] **Windows signing:** sign the Windows installer, for example with Azure Trusted Signing or a code-signing certificate. Without it, SmartScreen shows a warning.
- [ ] Publish the first release: tag `v0.1.0`, check the draft release, then publish it.

### 3. Make it open-source friendly

- [ ] Add `CONTRIBUTING.md`, a code of conduct, and issue templates (bug report: OS version, monitor setup, logs).
- [ ] Label starter tasks as `good first issue`.
- [ ] Add a screenshot or short GIF of the Settings window and a before/after restore to this README.

### 4. Features after v0.1

- [ ] **Auto-update** using Tauri's updater plugin.
- [ ] **Late-opening apps:** put an app's window in its saved spot when the app is opened after you've redocked.
- [ ] **Window stacking order:** restore which window was in front of which.
- [ ] **Virtual desktops:** support Windows virtual desktops and macOS Spaces.
- [ ] **Diagnostics:** an "Export diagnostics" button that zips the logs and the saved layouts for bug reports.
- [ ] **Package managers:** distribute through `winget` and a Homebrew cask.
- [ ] **Linux** support (X11 first).
- [ ] **Translations** of the Settings window.

## Install

There's no release yet. Once there is, installers will be on the [Releases](https://github.com/OBL-Y/Mooring/releases) page.

### Windows

1. Run `Mooring_x.y.z_x64-setup.exe`.
2. Choose **Only for me** (no admin rights; installs to `%LOCALAPPDATA%\Programs\Mooring`) or **Anyone who uses this computer** (needs admin; installs to `C:\Program Files\Mooring`).
3. On the last page, leave **Create desktop shortcut** ticked if you want one.

For a silent install, run the installer with `/S`.

To restore desktop icons, right-click the desktop › View and turn off **Auto arrange icons**. Otherwise Windows rearranges them itself.

### macOS

1. Open `Mooring_x.y.z_universal.dmg` and drag Mooring to Applications.
2. Open Mooring. When asked, turn it on under **System Settings › Privacy & Security › Accessibility**. Moving other apps' windows isn't possible without this.

### Settings

Open Settings from the tray or menu bar icon, or launch Mooring again (for example from the desktop icon).

| Setting | Default | What it does |
| --- | --- | --- |
| Restore automatically | On | Put windows back when screens change. |
| Also restore when undocking | On | Also restore the laptop-only layout when screens are removed. |
| Restore desktop icons | On | Windows only. |
| Run at startup / Open at login | On | Start Mooring quietly when you sign in. |
| Wait before restoring | 3 s | Increase it if your dock is slow to wake its screens. |

### Known limits

- macOS doesn't let apps place desktop icons, so the icon option only exists on Windows.
- Full-screen macOS windows have their own Space, so Mooring leaves them alone.
- On Windows, windows of apps running as administrator can't be moved unless Mooring also runs as administrator.
- A window that was closed and reopened gets its old spot back only if its app and title (or its app) match what was saved.

## How it works

Mooring is a [Tauri 2](https://v2.tauri.app) app: a Rust back end, a small HTML/JS Settings window, and the system's own web view. That keeps installers to a few MB.

1. **Watch the screens.** Once a second, Mooring reads the list of connected screens and builds a fingerprint from each screen's hardware id and position. Each fingerprint is one "display setup".
2. **Save.** While the setup is unchanged, it saves every app window's position, size and state every 5 seconds. Windows are only written to disk when something actually moved.
3. **Notice a change.** When the fingerprint changes, saving stops immediately, so the windows the OS just shuffled don't overwrite the good layout.
4. **Restore.** After the settle delay, it matches saved windows to open windows: by window id first, then by app and title, then by app. It moves each one, waits a moment, and retries any that didn't land.

How each OS does it:

| | Windows | macOS |
| --- | --- | --- |
| Screens | `EnumDisplayMonitors`, with EDID-based ids from `EnumDisplayDevices` | `CGGetActiveDisplayList` |
| Reading and moving windows | `EnumWindows`, `GetWindowPlacement`, `SetWindowPlacement` | Accessibility API (`AXUIElement`) |
| Desktop icons | The desktop's `IFolderView` (shell COM) | Not possible |
| Run at login | Per-user `Run` registry key | LaunchAgent |
| Installer | NSIS (per-user or per-machine) | DMG |

### Project layout

```
src-tauri/
  src/
    lib.rs             App start-up, tray menu, commands used by the Settings window
    engine.rs          Watch / save / restore loop
    model.rs           Data types, display fingerprint, window matching (+ unit tests)
    store.rs           Saved layouts (layouts.json)
    settings.rs        Settings (settings.json)
    platform/
      windows.rs       Win32 + desktop icons
      macos.rs         CoreGraphics + Accessibility
  tauri.conf.json      App and installer configuration
  windows/hooks.nsh    Extra Windows installer/uninstaller steps
ui/                    Settings window (plain HTML, CSS, JS, no build step)
scripts/make-icon.swift  Draws the app and tray icons
.github/workflows/     CI (tests on both OSes) and Release (builds installers)
```

## Contributing

Bug reports, testing on your own dock setup, and pull requests are all welcome. The fastest way to help right now is to pick an item from [step 1 of the Roadmap](#1-test-on-real-hardware-needed-for-v010), try it, and open an issue with what happened.

### 1. Install the tools

**Both systems**
- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) 20 or later

**Windows**
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **Desktop development with C++** workload
- WebView2. It's already installed on Windows 11.

**macOS**
- Xcode Command Line Tools: `xcode-select --install`

See Tauri's [prerequisites guide](https://v2.tauri.app/start/prerequisites/) if anything is missing.

### 2. Get the code and run it

```bash
git clone https://github.com/OBL-Y/Mooring.git
```

```bash
cd Mooring && npm install
```

```bash
npm run dev
```

On macOS, the development build runs from your terminal, so give **your terminal app** (or your editor) Accessibility access in System Settings › Privacy & Security › Accessibility.

Note: the first run turns on launch-at-login for the development build. Turn it off from the tray menu when you're done.

### 3. Check your change

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
```

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

To see what Mooring reads on your machine (screens, windows and icons), run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml live_read -- --ignored --nocapture
```

Logs are written to `~/Library/Logs/io.github.obl-y.mooring` on macOS and `%LOCALAPPDATA%\io.github.obl-y.mooring\logs` on Windows. Saved layouts and settings are in `~/Library/Application Support/io.github.obl-y.mooring` on macOS and `%APPDATA%\io.github.obl-y.mooring` on Windows.

### 4. Open a pull request

1. Fork the repo and create a branch from `main`.
2. Keep each PR to one change, and say in the description what you tested and on which OS and monitor setup.
3. CI runs the formatting check, clippy and the tests on both Windows and macOS. Please make sure it's green.

### Build the installers locally

```bash
npm run build
```

The output goes to `src-tauri/target/release/bundle/`: `nsis/` on Windows, and `macos/` and `dmg/` on macOS. To regenerate the icons, run `npm run icons` (needs macOS).

## Releasing

For maintainers:

1. Update the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
2. Commit, then tag and push: `git tag v0.1.0 && git push origin v0.1.0`.
3. The **Release** workflow builds the Windows installer and a universal macOS DMG and attaches them to a **draft** release.
4. Download and test both installers, then publish the release.

For test builds, use a tag with a suffix, such as `v0.1.0-beta.1`. These are published straight away as a **pre-release**, so testers can download the installers without signing in.

To sign the Mac build, add these repository secrets: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID`. For Windows signing, see the [Tauri Windows signing guide](https://v2.tauri.app/distribute/sign/windows/).

## License

[MIT](LICENSE) © 2026 OBL-Y
