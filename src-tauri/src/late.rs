//! Placing windows from apps opened after a restore.
//!
//! When a layout is restored, saved windows whose app isn't running can't be
//! moved. `LateWatch` remembers them, and for a while afterwards matches each
//! newly opened window against them, so a late-starting app still lands in its
//! saved spot. Each saved window is used once, and each new window is only
//! looked at once, so Mooring never fights someone moving a window themselves.

use crate::model::{pair_windows, WindowState};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// How long after a restore late windows are placed.
pub const WATCH_FOR: Duration = Duration::from_secs(30 * 60);
/// How long a new window must exist before it's moved. Apps often position
/// their own windows as they open; moving before that gets undone.
pub const SETTLE: Duration = Duration::from_millis(1500);

pub struct LateWatch {
    /// The display setup the restore was for.
    pub key: String,
    /// Saved windows that weren't open at restore time and haven't been placed.
    targets: Vec<WindowState>,
    /// Windows already open at restore, or already considered.
    known: HashSet<u64>,
    /// New windows, and when they were first seen.
    first_seen: HashMap<u64, Instant>,
    until: Instant,
}

impl LateWatch {
    /// `missing` are the saved windows that weren't open; `open` is every window
    /// open right after the restore.
    pub fn new(key: String, missing: Vec<WindowState>, open: &[WindowState], now: Instant) -> Self {
        Self {
            key,
            targets: missing,
            known: open.iter().map(|w| w.id).collect(),
            first_seen: HashMap::new(),
            until: now + WATCH_FOR,
        }
    }

    /// True once there's nothing left to place, or time is up.
    pub fn finished(&self, now: Instant) -> bool {
        self.targets.is_empty() || now >= self.until
    }

    /// Looks at the windows open now. Returns (saved spot, window) for each new
    /// window that has settled and matches a saved window.
    pub fn observe(
        &mut self,
        current: &[WindowState],
        now: Instant,
    ) -> Vec<(WindowState, WindowState)> {
        let open: HashSet<u64> = current.iter().map(|w| w.id).collect();
        self.first_seen.retain(|id, _| open.contains(id));

        let mut ready = Vec::new();
        for window in current {
            if self.known.contains(&window.id) {
                continue;
            }
            let seen = *self.first_seen.entry(window.id).or_insert(now);
            if now.duration_since(seen) >= SETTLE {
                ready.push(window.clone());
            }
        }
        if ready.is_empty() {
            return Vec::new();
        }
        for window in &ready {
            self.known.insert(window.id);
            self.first_seen.remove(&window.id);
        }

        let mut pairs = pair_windows(&self.targets, &ready);
        // Remove the used targets from the back so the indices stay valid.
        pairs.sort_by_key(|p| std::cmp::Reverse(p.0));
        let mut placed: Vec<(WindowState, WindowState)> = pairs
            .into_iter()
            .map(|(ti, wi)| (self.targets.remove(ti), ready[wi].clone()))
            .collect();
        placed.reverse();
        placed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WinState;

    fn win(id: u64, app: &str, title: &str, x: i32) -> WindowState {
        WindowState {
            id,
            pid: 1,
            app: app.into(),
            title: title.into(),
            class: "c".into(),
            x,
            y: 0,
            width: 100,
            height: 100,
            state: WinState::Normal,
            order: 0,
        }
    }

    #[test]
    fn places_a_late_window_once_it_settles() {
        let t0 = Instant::now();
        let saved = vec![win(0, "Outlook", "Inbox", 2000)];
        let open = vec![win(1, "Safari", "News", 0)];
        let mut watch = LateWatch::new("k".into(), saved, &open, t0);

        let now = vec![win(1, "Safari", "News", 0), win(7, "Outlook", "Inbox", 0)];
        // Just appeared: not yet.
        assert!(watch.observe(&now, t0).is_empty());
        // Settled: placed at the saved spot.
        let placed = watch.observe(&now, t0 + SETTLE);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].0.x, 2000);
        assert_eq!(placed[0].1.id, 7);
        assert!(watch.finished(t0 + SETTLE));
    }

    #[test]
    fn leaves_already_open_and_unrelated_windows_alone() {
        let t0 = Instant::now();
        let saved = vec![win(0, "Outlook", "Inbox", 2000)];
        // Outlook was already open (in another window) when restoring.
        let open = vec![win(1, "Outlook", "Calendar", 0)];
        let mut watch = LateWatch::new("k".into(), saved, &open, t0);

        let now = vec![
            win(1, "Outlook", "Calendar", 0),
            win(9, "Notes", "Inbox", 0),
        ];
        watch.observe(&now, t0);
        assert!(watch.observe(&now, t0 + SETTLE).is_empty());
        assert!(!watch.finished(t0 + SETTLE));
    }

    #[test]
    fn looks_at_each_new_window_only_once() {
        let t0 = Instant::now();
        let saved = vec![
            win(0, "Outlook", "Inbox", 2000),
            win(0, "Outlook", "Inbox", 3000),
        ];
        let mut watch = LateWatch::new("k".into(), saved, &[], t0);

        let now = vec![win(7, "Outlook", "Inbox", 0)];
        watch.observe(&now, t0);
        assert_eq!(watch.observe(&now, t0 + SETTLE).len(), 1);
        // The same window again later: already handled, even though a saved spot is left.
        assert!(watch.observe(&now, t0 + SETTLE * 4).is_empty());
    }

    #[test]
    fn stops_after_the_watch_time() {
        let t0 = Instant::now();
        let watch = LateWatch::new("k".into(), vec![win(0, "a", "b", 0)], &[], t0);
        assert!(!watch.finished(t0));
        assert!(watch.finished(t0 + WATCH_FOR));
    }
}
