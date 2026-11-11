# Keep each screen's wallpaper

**Status:** planned for v0.2.0 (not started). Discussion and progress: [#4](https://github.com/OBL-Y/Mooring/issues/4).

## The problem

People who use a different wallpaper on each monitor find Windows and macOS resetting them:

- **After redocking**, when the OS treats a monitor that comes back through a different dock or port as a new one.
- **After a reboot or an OS update**, sometimes before any app is running.

Resets take two forms:

- A screen goes back to a **system default** picture.
- A screen **collapses to match another screen**. For example, screens 1 and 2 share a picture and the portrait screen 3 has its own; after a reboot, screen 3 shows screen 1's picture too.

Getting them back means going into Settings and setting each screen again.

## The idea

Mooring already recognises each physical monitor by its hardware id and already handles display changes. It can also remember each monitor's wallpaper and put it back when the OS resets it.

A wallpaper belongs to the **physical monitor**, not to a display setup: a given monitor gets the same picture at home with one screen or at the office with two.

## Telling a reset from a real change

A wallpaper change is treated as **a reset to undo** (not a new choice to remember) when any of these is true:

| Pattern | Example |
| --- | --- |
| It happens around a display change (dock, undock, wake) | Same settle-and-restore timing as windows |
| It happened while Mooring wasn't running: at start-up, a screen differs from what Mooring saved before the restart | Reset at boot or after an update |
| A screen collapses to match another screen that used to differ | Portrait screen 3 suddenly matches screen 1 |
| A screen switches to a system default picture | `C:\Windows\Web\Wallpaper\…`, `/System/Library/Desktop Pictures/…` |

Anything else, while the screens are stable, is the person changing their wallpaper, and Mooring remembers it.

**Lock** freezes the saved wallpapers: nothing changes them except the person, in Mooring.

## Phase 1: keep wallpapers set (both systems)

- Remember each monitor's wallpaper picture and fit (fill, fit, centre, …), keyed by the monitor's hardware id.
- Keep Mooring's own copy of each picture in its data folder, so it works even if the original file is moved, deleted or cleaned up.
- Persist saved wallpapers across restarts, and check every screen against them at start-up.
- Restore after a display change settles, at start-up, and when the reset patterns above are seen.
- Settings: **Keep each screen's wallpaper** (on by default) and **Lock wallpapers**.

### Leave alone

- **Slideshow and Windows Spotlight**: Mooring doesn't stop or replace them.
- **Policy-managed wallpapers** (set by an organisation): never touched.
- **macOS moving wallpapers** (Aerials, shuffle): can't be stored. Dynamic (time-of-day) wallpapers are files and should work; to be confirmed.

### Must handle

- **Portrait and mixed orientations**: the fit is stored per screen.
- **Identical monitors** (same model): told apart by serial number, with position as a fallback when a monitor reports no serial.
- **macOS Spaces**: Phase 1 keeps the wallpaper of the Space currently shown on each screen.

### How

- **Windows:** `IDesktopWallpaper` (COM, public): `GetMonitorDevicePathAt`, `GetWallpaper`, `SetWallpaper`, `GetPosition`/`SetPosition`, `GetSlideshow`/`GetStatus` to detect slideshows. Map its monitor device paths to Mooring's EDID-based display ids. Skip when the `Wallpaper` policy is set under `Software\Microsoft\Windows\CurrentVersion\Policies\System`.
- **macOS:** `NSWorkspace.desktopImageURL(for:)` and `setDesktopImageURL(_:for:options:)` per `NSScreen`, with the scaling and fill-colour options. Map `NSScreen` to `CGDirectDisplayID` to Mooring's display id.
- **Engine:** a per-monitor wallpaper store alongside the layout store; read wallpapers during stable snapshots, freeze during display changes (like window snapshots), and restore after the settle delay and at start-up.

## Phase 2: Wallpapers in Settings

- A **Wallpapers** section listing each monitor with a preview of its wallpaper and a status: *Kept*, *Locked*, *Slideshow: left alone*, *Can't keep moving wallpapers*, *Managed by your organisation*.
- A **Choose…** button per monitor to set its wallpaper from Mooring.

## Phase 3 (macOS): separate wallpapers per Space

- Remember a wallpaper per screen per Space, and restore it as each Space is visited.
- Apple's public API can't tell which Space is showing. This would use the undocumented, read-only `CGSCopyManagedDisplaySpaces` / `CGSGetActiveSpace` calls (space UUIDs persist across restarts). They work without turning off System Integrity Protection, and Mooring already uses one undocumented call (`_AXUIElementGetWindow`). Riskiest phase, so it comes last.

## Testing

Each phase needs real setups on both systems: dock and undock, reboot, and ideally an OS update cycle. Cases to cover:

- Two screens sharing a picture plus a portrait screen with its own.
- Identical monitors.
- A slideshow user (must be left alone).
- Mac with several Spaces.

## Release

Phases 1 and 2 together as **v0.2.0**, after v0.1.1 has tested the release pipeline.
