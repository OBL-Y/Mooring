use crate::late::LateWatch;
use crate::model::{display_key, display_label, pair_windows, Layout, WindowState};
use crate::platform;
use crate::settings::Settings;
use crate::store::Store;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const POLL: Duration = Duration::from_millis(1000);
const RETRY_DELAY: Duration = Duration::from_millis(600);

/// Watches the display setup, keeps a layout snapshot for each setup,
/// and restores the matching layout when the setup changes.
pub struct Engine {
    data_dir: PathBuf,
    settings: Mutex<Settings>,
    store: Mutex<Store>,
    track: Mutex<Track>,
    /// Held while snapshotting or restoring so the two never overlap.
    /// When both are needed, take `busy` before `late`.
    busy: Mutex<()>,
    /// Saved windows from the last restore whose apps weren't open yet.
    late: Mutex<Option<LateWatch>>,
}

#[derive(Default)]
struct Track {
    key: String,
    label: String,
    display_count: usize,
    previous_count: usize,
    /// Set when the setup changed and we are waiting for it to settle.
    changed_at: Option<Instant>,
    last_snapshot: Option<Instant>,
    last_event: String,
    last_event_at: u64,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct RestoreReport {
    pub moved: usize,
    pub failed: usize,
    pub missing: usize,
    pub icons: usize,
}

#[derive(Serialize)]
pub struct LayoutSummary {
    pub key: String,
    pub label: String,
    pub saved_at: u64,
    pub pinned: bool,
    pub window_count: usize,
    pub icon_count: usize,
    pub current: bool,
}

#[derive(Serialize)]
pub struct Status {
    pub platform: &'static str,
    pub has_permission: bool,
    pub supports_icons: bool,
    pub current_label: String,
    pub last_event: String,
    pub last_event_at: u64,
    pub layouts: Vec<LayoutSummary>,
}

pub(crate) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Engine {
    pub fn new(data_dir: PathBuf, settings: Settings) -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(Store::open(data_dir.clone())),
            data_dir,
            settings: Mutex::new(settings),
            track: Mutex::new(Track::default()),
            busy: Mutex::new(()),
            late: Mutex::new(None),
        })
    }

    pub fn start(self: &Arc<Self>) {
        let engine = Arc::clone(self);
        std::thread::Builder::new()
            .name("mooring-engine".into())
            .spawn(move || loop {
                engine.tick();
                std::thread::sleep(POLL);
            })
            .expect("failed to start engine thread");
    }

    pub fn settings(&self) -> Settings {
        lock(&self.settings).clone()
    }

    pub fn set_settings(&self, settings: Settings) -> Result<Settings, String> {
        let settings = settings.clamp();
        settings.save(&self.data_dir)?;
        *lock(&self.settings) = settings.clone();
        Ok(settings)
    }

    fn note(&self, event: impl Into<String>) {
        let event = event.into();
        log::info!("{event}");
        let mut t = lock(&self.track);
        t.last_event = event;
        t.last_event_at = now_secs();
    }

    fn tick(&self) {
        let displays = platform::displays();
        // No displays usually means the lid is shut or the machine is going to sleep.
        if displays.is_empty() {
            return;
        }
        let key = display_key(&displays);
        let settings = self.settings();

        let mut t = lock(&self.track);
        if key != t.key {
            let first = t.key.is_empty();
            t.previous_count = if first {
                displays.len()
            } else {
                t.display_count
            };
            t.key = key;
            t.label = display_label(&displays);
            t.display_count = displays.len();
            drop(t);
            // Late windows belong to the old setup.
            *lock(&self.late) = None;
            let mut t = lock(&self.track);
            if first {
                t.last_event = format!("Watching {}", t.label);
                t.last_event_at = now_secs();
            } else {
                // Wait for the setup to settle; another change restarts the wait.
                t.changed_at = Some(Instant::now());
                t.last_event = format!("Displays changed: {}", t.label);
                t.last_event_at = now_secs();
            }
            return;
        }

        if let Some(changed_at) = t.changed_at {
            if changed_at.elapsed() < Duration::from_secs(settings.settle_delay_secs) {
                return;
            }
            t.changed_at = None;
            // Don't snapshot straight away: give the restore (or the user) a moment.
            t.last_snapshot = Some(Instant::now());
            let undocking = t.display_count < t.previous_count;
            let label = t.label.clone();
            drop(t);

            if !settings.auto_restore {
                self.note(format!(
                    "Auto-restore is off; not moving windows for {label}"
                ));
            } else if undocking && !settings.restore_on_undock {
                self.note("Undocked; restoring on undock is off");
            } else {
                let layout = lock(&self.store).get(&key).cloned();
                match layout {
                    Some(layout) => {
                        let report = self.restore(&layout);
                        self.note(describe_restore(&report));
                    }
                    None => self.note(format!("New display setup, learning its layout: {label}")),
                }
            }
            return;
        }

        drop(t);
        self.place_late_windows(&key);

        let t = lock(&self.track);
        let due = t
            .last_snapshot
            .is_none_or(|at| at.elapsed() >= Duration::from_secs(settings.snapshot_interval_secs));
        drop(t);
        if due {
            if let Err(e) = self.snapshot(false) {
                log::debug!("snapshot skipped: {e}");
            }
        }
    }

    /// Saves the current window positions for the current display setup.
    /// Returns the number of windows saved.
    pub fn snapshot(&self, manual: bool) -> Result<usize, String> {
        let _busy = lock(&self.busy);
        let displays = platform::displays();
        let key = display_key(&displays);
        {
            let mut t = lock(&self.track);
            t.last_snapshot = Some(Instant::now());
            if displays.is_empty() || key != t.key || t.changed_at.is_some() {
                return Err("Displays are changing; try again in a moment".into());
            }
        }
        if !manual && lock(&self.store).get(&key).is_some_and(|l| l.pinned) {
            return Ok(0);
        }

        let settings = self.settings();
        let mut windows = platform::windows();
        if let Some(saved) = lock(&self.store).get(&key) {
            windows.extend(keep_out_of_sight(
                &saved.windows,
                &windows,
                &platform::existing_windows(),
            ));
        }
        let icons = if platform::SUPPORTS_ICONS && settings.restore_desktop_icons {
            platform::desktop_icons()
        } else {
            Vec::new()
        };

        // If the displays changed while we were reading, these positions are
        // already being shuffled by the OS. Throw them away.
        if display_key(&platform::displays()) != key {
            return Err("Displays changed during snapshot".into());
        }
        if windows.is_empty() && !manual {
            // Locked screen or missing permission: don't wipe a good layout.
            return Ok(0);
        }

        let count = windows.len();
        let changed = lock(&self.store).put(Layout {
            label: display_label(&displays),
            display_count: displays.len(),
            key,
            saved_at: now_secs(),
            pinned: false,
            windows,
            icons,
        });
        if manual {
            self.note(format!("Saved {count} windows"));
        } else if changed {
            log::debug!("layout updated ({count} windows)");
        }
        Ok(count)
    }

    /// Restores the saved layout for the current display setup.
    pub fn restore_now(&self) -> Result<RestoreReport, String> {
        let key = display_key(&platform::displays());
        let layout = lock(&self.store)
            .get(&key)
            .cloned()
            .ok_or("Nothing saved for this display setup yet")?;
        let report = self.restore(&layout);
        self.note(describe_restore(&report));
        Ok(report)
    }

    fn restore(&self, layout: &Layout) -> RestoreReport {
        let _busy = lock(&self.busy);
        let mut report = RestoreReport::default();
        let current = platform::windows();
        let pairs = pair_windows(&layout.windows, &current);
        report.missing = layout.windows.len() - pairs.len();

        // Remember the windows whose apps aren't open yet, to place them when they open.
        let mut matched = vec![false; layout.windows.len()];
        for &(si, _) in &pairs {
            matched[si] = true;
        }
        let missing: Vec<WindowState> = layout
            .windows
            .iter()
            .zip(&matched)
            .filter(|(_, &m)| !m)
            .map(|(w, _)| w.clone())
            .collect();
        *lock(&self.late) = (self.settings().place_late_windows && !missing.is_empty())
            .then(|| LateWatch::new(layout.key.clone(), missing, &current, Instant::now()));

        let mut moved: Vec<(&WindowState, u64)> = Vec::new();
        for &(si, ci) in &pairs {
            let (target, now) = (&layout.windows[si], &current[ci]);
            if target.same_place(now) {
                continue;
            }
            match platform::apply(target, now) {
                Ok(()) => moved.push((target, now.id)),
                Err(e) => {
                    log::warn!("could not move {} '{}': {e}", now.app, now.title);
                    report.failed += 1;
                }
            }
        }

        // Some apps resist the first move (or the OS nudges them back while
        // displays finish arranging). Check once more and retry stragglers.
        if !moved.is_empty() {
            std::thread::sleep(RETRY_DELAY);
            let after = platform::windows();
            for (target, id) in moved {
                match after.iter().find(|w| w.id == id) {
                    Some(now) if !target.same_place(now) => {
                        if platform::apply(target, now).is_ok() {
                            report.moved += 1;
                        } else {
                            report.failed += 1;
                        }
                    }
                    _ => report.moved += 1,
                }
            }
        }

        if self.settings().restore_stacking && pairs.len() > 1 {
            // Front-to-back as saved, using the windows as they are now.
            let mut stack: Vec<(u32, WindowState)> = pairs
                .iter()
                .map(|&(si, ci)| {
                    let target = &layout.windows[si];
                    let mut window = current[ci].clone();
                    window.state = target.state;
                    (target.order, window)
                })
                .collect();
            stack.sort_by_key(|(order, _)| *order);
            let stack: Vec<WindowState> = stack.into_iter().map(|(_, w)| w).collect();
            let stacked = platform::set_stacking(&stack);
            log::info!("restored stacking order of {stacked} windows");
        }

        if platform::SUPPORTS_ICONS
            && self.settings().restore_desktop_icons
            && !layout.icons.is_empty()
        {
            match platform::set_desktop_icons(&layout.icons) {
                Ok(n) => report.icons = n,
                Err(e) => log::warn!("could not restore desktop icons: {e}"),
            }
        }
        report
    }

    /// Moves newly opened windows that match a saved window from the last restore.
    fn place_late_windows(&self, key: &str) {
        if lock(&self.late).is_none() {
            return;
        }
        let _busy = lock(&self.busy);
        let placed = {
            let mut late = lock(&self.late);
            let Some(watch) = late.as_mut() else {
                return;
            };
            let now = Instant::now();
            if watch.key != key || !self.settings().place_late_windows {
                *late = None;
                return;
            }
            let placed = watch.observe(&platform::windows(), now);
            if watch.finished(now) {
                *late = None;
            }
            placed
        };

        for (target, window) in placed {
            if target.same_place(&window) {
                continue;
            }
            if let Err(e) = platform::apply(&target, &window) {
                log::warn!("could not place {} '{}': {e}", window.app, window.title);
                continue;
            }
            // Apps that are still starting up sometimes move themselves once more.
            std::thread::sleep(RETRY_DELAY);
            if let Some(now) = platform::windows().iter().find(|w| w.id == window.id) {
                if !target.same_place(now) {
                    let _ = platform::apply(&target, now);
                }
            }
            self.note(format!(
                "Placed {} in its saved spot",
                app_name(&window.app)
            ));
        }
    }

    pub fn set_pinned(&self, key: &str, pinned: bool) -> bool {
        lock(&self.store).set_pinned(key, pinned)
    }

    pub fn forget(&self, key: &str) -> bool {
        lock(&self.store).remove(key)
    }

    pub fn status(&self) -> Status {
        let t = lock(&self.track);
        let layouts = lock(&self.store)
            .all()
            .map(|l| LayoutSummary {
                key: l.key.clone(),
                label: l.label.clone(),
                saved_at: l.saved_at,
                pinned: l.pinned,
                window_count: l.windows.len(),
                icon_count: l.icons.len(),
                current: l.key == t.key,
            })
            .collect();
        Status {
            platform: platform::NAME,
            has_permission: platform::has_permission(),
            supports_icons: platform::SUPPORTS_ICONS,
            current_label: t.label.clone(),
            last_event: t.last_event.clone(),
            last_event_at: t.last_event_at,
            layouts,
        }
    }
}

/// Saved windows that still exist but that Mooring can't see right now: on
/// another macOS Space, for example. Keeping them means their spot isn't
/// forgotten, so a restore can put them back once they're in view.
fn keep_out_of_sight(
    saved: &[WindowState],
    seen: &[WindowState],
    existing: &HashSet<u64>,
) -> Vec<WindowState> {
    let seen: HashSet<u64> = seen.iter().map(|w| w.id).collect();
    saved
        .iter()
        .filter(|w| w.id != 0 && !seen.contains(&w.id) && existing.contains(&w.id))
        .cloned()
        .collect()
}

/// A short app name: on Windows `app` is the program's full path.
fn app_name(app: &str) -> &str {
    let name = app.rsplit(['\\', '/']).next().unwrap_or(app);
    name.strip_suffix(".exe")
        .or_else(|| name.strip_suffix(".EXE"))
        .unwrap_or(name)
}

fn describe_restore(r: &RestoreReport) -> String {
    let mut text = match r.moved {
        0 => "Restored layout (windows were already in place)".to_string(),
        1 => "Restored 1 window".to_string(),
        n => format!("Restored {n} windows"),
    };
    if r.icons > 0 {
        text.push_str(&format!(", {} desktop icons", r.icons));
    }
    if r.failed > 0 {
        text.push_str(&format!("; {} couldn't be moved", r.failed));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WinState;

    fn win(id: u64) -> WindowState {
        WindowState {
            id,
            pid: 1,
            app: "a".into(),
            title: format!("w{id}"),
            class: "c".into(),
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            state: WinState::Normal,
            order: 0,
        }
    }

    #[test]
    fn keeps_only_saved_windows_that_exist_out_of_sight() {
        let saved = vec![win(1), win(2), win(3), win(0)];
        let seen = vec![win(1)];
        // 2 is on another Space; 3 was closed; 0 has no id.
        let existing: HashSet<u64> = [1, 2].into_iter().collect();
        let kept = keep_out_of_sight(&saved, &seen, &existing);
        assert_eq!(kept.iter().map(|w| w.id).collect::<Vec<_>>(), vec![2]);
    }

    #[test]
    fn app_names_drop_the_path_and_exe() {
        assert_eq!(
            app_name(r"C:\Program Files\Microsoft Office\OUTLOOK.EXE"),
            "OUTLOOK"
        );
        assert_eq!(app_name("Notes"), "Notes");
    }
}
