# Contributing to Mooring

Thanks for helping. Mooring is small, and every bug report, test on a new dock setup, and pull request makes a real difference.

Everyone taking part is expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

The most useful thing right now is testing on real hardware. Pick an open item from [step 1 of the Roadmap](README.md#1-test-on-real-hardware-needed-for-v010), try it, and open an issue with what happened. That counts even if it all worked.

## Reporting a bug

[Open a bug report](https://github.com/OBL-Y/Mooring/issues/new?template=bug_report.yml). The form asks for:

- **Your system:** Windows 10, Windows 11, or the macOS version, plus the Mooring version (from the release you installed).
- **Your screens:** the laptop screen and each monitor, with resolution and scaling (for example "laptop 1920×1200 at 150%, two 2560×1440 at 100% over USB-C dock").
- **What you did:** for example "undocked, closed the lid, redocked an hour later".
- **What happened, and what you expected.** Which windows went to the wrong place, if any.
- **The log.** It's on Windows in `%LOCALAPPDATA%\io.github.obl-y.mooring\logs`, and on macOS in `~/Library/Logs/io.github.obl-y.mooring`.

If it's about where windows ended up, the saved layouts help too: `layouts.json` in `%APPDATA%\io.github.obl-y.mooring` (Windows) or `~/Library/Application Support/io.github.obl-y.mooring` (macOS). It includes window titles, so check it for anything private before attaching it.

## Suggesting a feature

Check the [Roadmap](README.md#roadmap) first. If your idea isn't there, [open a feature request](https://github.com/OBL-Y/Mooring/issues/new?template=feature_request.yml) that says what problem it solves for you. That's easier to discuss than a finished design.

## Working on the code

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

The [project layout](README.md#project-layout) in the README says which file does what.

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

If you changed the Settings window, update the README screenshots (needs macOS):

```bash
npm run screenshots
```

### 4. Open a pull request

1. Fork the repo and create a branch from `main`.
2. Keep each PR to one change, and say in the description what you tested and on which OS and monitor setup.
3. CI runs the formatting check, clippy and the tests on both Windows and macOS. Please make sure it's green.

Code that only runs on one OS lives in `src-tauri/src/platform/`. If you change one platform's behaviour, check whether the other needs the same change, or say in the PR that it doesn't.

### Build the installers locally

```bash
npm run build
```

The output goes to `src-tauri/target/release/bundle/`: `nsis/` on Windows, and `macos/` and `dmg/` on macOS. To regenerate the icons, run `npm run icons` (needs macOS).

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
