use serde::{Deserialize, Serialize};

/// One connected display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Display {
    /// Stable hardware id (EDID-based where the OS gives one).
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub primary: bool,
    pub builtin: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WinState {
    Normal,
    Maximized,
    Minimized,
}

/// A top-level window and where it sits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowState {
    /// HWND on Windows, CGWindowID on macOS. Only valid while the window exists.
    pub id: u64,
    pub pid: u32,
    /// Executable path on Windows, application name on macOS.
    pub app: String,
    pub title: String,
    /// Window class on Windows, AX subrole on macOS.
    pub class: String,
    /// The restored (non-maximized) rectangle, in the OS's own coordinates.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub state: WinState,
    /// Front-to-back position at the time of the snapshot.
    pub order: u32,
}

impl WindowState {
    pub fn same_place(&self, other: &WindowState) -> bool {
        const TOLERANCE: i32 = 2;
        self.state == other.state
            && (self.x - other.x).abs() <= TOLERANCE
            && (self.y - other.y).abs() <= TOLERANCE
            && (self.width - other.width).abs() <= TOLERANCE
            && (self.height - other.height).abs() <= TOLERANCE
    }
}

/// A desktop icon and its position (Windows only).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IconState {
    /// Parsing name (full path) of the item.
    pub name: String,
    pub x: i32,
    pub y: i32,
}

/// Everything saved for one display setup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub key: String,
    pub label: String,
    pub display_count: usize,
    /// Unix seconds when the layout last changed.
    pub saved_at: u64,
    /// When pinned, automatic snapshots stop overwriting this layout.
    #[serde(default)]
    pub pinned: bool,
    pub windows: Vec<WindowState>,
    #[serde(default)]
    pub icons: Vec<IconState>,
}

/// A fingerprint of the connected displays, so each setup gets its own layout.
pub fn display_key(displays: &[Display]) -> String {
    let mut parts: Vec<String> = displays
        .iter()
        .map(|d| format!("{}@{},{},{}x{}", d.id, d.x, d.y, d.width, d.height))
        .collect();
    parts.sort();
    parts.join("|")
}

/// A human-readable description of a display setup.
pub fn display_label(displays: &[Display]) -> String {
    if displays.is_empty() {
        return "No displays".into();
    }
    let mut sorted: Vec<&Display> = displays.iter().collect();
    sorted.sort_by_key(|d| (d.x, d.y));
    let names: Vec<String> = sorted
        .iter()
        .map(|d| {
            if d.builtin {
                format!("Built-in {}×{}", d.width, d.height)
            } else {
                format!("{}×{}", d.width, d.height)
            }
        })
        .collect();
    let count = displays.len();
    let noun = if count == 1 { "display" } else { "displays" };
    format!("{count} {noun}: {}", names.join(", "))
}

/// Pairs saved windows with the windows open now. Returns (saved index, current index).
///
/// Tries, in order: the same live window id, then the same app and title,
/// then the same app and window class (in front-to-back order).
pub fn pair_windows(saved: &[WindowState], current: &[WindowState]) -> Vec<(usize, usize)> {
    type Rule = fn(&WindowState, &WindowState) -> bool;
    let rules: [Rule; 3] = [
        |s, c| s.id != 0 && s.id == c.id && s.app == c.app,
        |s, c| s.app == c.app && !s.title.is_empty() && s.title == c.title,
        |s, c| s.app == c.app && s.class == c.class,
    ];

    let mut saved_order: Vec<usize> = (0..saved.len()).collect();
    saved_order.sort_by_key(|&i| saved[i].order);
    let mut current_order: Vec<usize> = (0..current.len()).collect();
    current_order.sort_by_key(|&i| current[i].order);

    let mut saved_used = vec![false; saved.len()];
    let mut current_used = vec![false; current.len()];
    let mut pairs = Vec::new();
    for rule in rules {
        for &si in &saved_order {
            if saved_used[si] {
                continue;
            }
            let found = current_order
                .iter()
                .copied()
                .find(|&ci| !current_used[ci] && rule(&saved[si], &current[ci]));
            if let Some(ci) = found {
                saved_used[si] = true;
                current_used[ci] = true;
                pairs.push((si, ci));
            }
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(id: u64, app: &str, title: &str, order: u32) -> WindowState {
        WindowState {
            id,
            pid: 1,
            app: app.into(),
            title: title.into(),
            class: "c".into(),
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            state: WinState::Normal,
            order,
        }
    }

    #[test]
    fn pairs_by_id_first() {
        let saved = vec![win(1, "a", "One", 0), win(2, "a", "Two", 1)];
        let current = vec![win(2, "a", "Renamed", 0), win(1, "a", "One", 1)];
        let mut pairs = pair_windows(&saved, &current);
        pairs.sort();
        assert_eq!(pairs, vec![(0, 1), (1, 0)]);
    }

    #[test]
    fn falls_back_to_title_then_class() {
        // Ids changed (e.g. after a restart).
        let saved = vec![
            win(1, "a", "Inbox", 0),
            win(2, "a", "Draft", 1),
            win(3, "b", "X", 2),
        ];
        let current = vec![win(10, "a", "Something else", 0), win(11, "a", "Inbox", 1)];
        let mut pairs = pair_windows(&saved, &current);
        pairs.sort();
        assert_eq!(pairs, vec![(0, 1), (1, 0)]);
    }

    #[test]
    fn never_pairs_across_apps() {
        let saved = vec![win(1, "a", "Same", 0)];
        let current = vec![win(1, "b", "Same", 0)];
        assert!(pair_windows(&saved, &current).is_empty());
    }

    #[test]
    fn display_key_ignores_order() {
        let d = |id: &str, x| Display {
            id: id.into(),
            name: id.into(),
            x,
            y: 0,
            width: 10,
            height: 10,
            primary: false,
            builtin: false,
        };
        assert_eq!(
            display_key(&[d("a", 0), d("b", 10)]),
            display_key(&[d("b", 10), d("a", 0)])
        );
        assert_ne!(
            display_key(&[d("a", 0)]),
            display_key(&[d("a", 0), d("b", 10)])
        );
    }
}
