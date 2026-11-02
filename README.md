# Mooring

A boat moors at a dock and returns to the same spot, which is what your windows do when you redock.

Mooring runs quietly in the system tray (Windows) or menu bar (macOS). It remembers where every window sits for each display setup — docked at your desk, laptop on its own, the meeting-room screen — and puts them back when you plug in or unplug. On Windows it also keeps your desktop icons in the order you left them.

## How it works

- Every few seconds Mooring saves the position, size and state (normal / maximized / minimized) of your app windows, filed under the current set of displays.
- When the displays change, it stops saving, waits for the screens to settle (3 seconds by default), and moves each window back to where it was the last time you used that setup.
- Windows are matched to their saved spot by window id, then by app and title, then by app.
- **Lock** a layout in Settings to keep it fixed; only **Save Current Layout** will change it then.

## Install

Download the latest installer from [Releases](https://github.com/OBL-Y/Mooring/releases).

### Windows

Run `Mooring_x.y.z_x64-setup.exe`. The installer asks whether to install:

- **Only for me**: no admin rights needed. Installs to `%LOCALAPPDATA%\Programs\Mooring`.
- **For anyone using this computer**: needs admin. Installs to `C:\Program Files\Mooring`.

It adds a Start menu entry, and the last page has a **Create desktop shortcut** box (ticked by default). For a silent install, run it with `/S`.

Mooring turns on **Run at startup** for each user the first time it runs. Turn it off from the tray menu or in Settings.

To restore desktop icons, right-click the desktop › View and turn off **Auto arrange icons**. Otherwise Windows rearranges them itself.

### macOS

Open `Mooring_x.y.z_universal.dmg` and drag Mooring to Applications. The first time it runs, macOS asks you to allow it under **System Settings › Privacy & Security › Accessibility**. Moving other apps' windows isn't possible without this permission.

**Open at Login** is on by default. Turn it off from the menu bar icon or in Settings.

macOS doesn't let apps set where desktop icons sit, so on a Mac the icon option is hidden. Full-screen windows have their own Space, so Mooring leaves them alone.

## Settings

Open Settings from the tray or menu bar icon, or launch Mooring again (for example from the desktop icon).

| Setting | Default | What it does |
| --- | --- | --- |
| Restore automatically | On | Put windows back when displays change. |
| Also restore when undocking | On | Also restore the laptop-only layout when screens are removed. |
| Restore desktop icons | On | Windows only. |
| Run at startup / Open at login | On | Start Mooring when you sign in. |
| Wait before restoring | 3 s | Increase it if your dock is slow to wake its displays. |

Settings and layouts are stored in `%APPDATA%\io.github.obl-y.mooring` (Windows) or `~/Library/Application Support/io.github.obl-y.mooring` (macOS).

## Limits

- On Windows, Mooring can't move windows of apps running as administrator unless Mooring is also run as administrator.
- A window that was closed and reopened gets its old spot back only if its app and title (or app) match what was saved.

## Build from source

You need [Rust](https://rustup.rs) and Node.js.

```bash
npm install
npm run dev      # run with live logs
npm run build    # build installers into src-tauri/target/release/bundle
```

Run the tests with `cargo test --manifest-path src-tauri/Cargo.toml`. To regenerate the icons, run `npm run icons` (needs macOS).

### Releasing

Push a tag such as `v0.1.0`. The **Release** workflow builds the Windows installer and a universal macOS DMG and attaches them to a draft GitHub release.

Without signing, Windows SmartScreen and macOS Gatekeeper will warn on first launch. To sign the Mac build, add these repository secrets: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID`. Windows signing needs a code-signing certificate (see the [Tauri signing guide](https://v2.tauri.app/distribute/sign/windows/)).

## License

MIT
