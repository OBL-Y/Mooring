use crate::model::Layout;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Saved layouts, one per display setup, kept in `layouts.json`.
pub struct Store {
    path: PathBuf,
    layouts: BTreeMap<String, Layout>,
}

impl Store {
    pub fn open(dir: PathBuf) -> Self {
        let path = dir.join("layouts.json");
        let layouts = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self { path, layouts }
    }

    pub fn get(&self, key: &str) -> Option<&Layout> {
        self.layouts.get(key)
    }

    pub fn all(&self) -> impl Iterator<Item = &Layout> {
        self.layouts.values()
    }

    /// Stores the layout, writing to disk only if windows or icons actually changed.
    /// Keeps the existing pinned flag. Returns true when something changed.
    pub fn put(&mut self, mut layout: Layout) -> bool {
        if let Some(old) = self.layouts.get(&layout.key) {
            layout.pinned = old.pinned;
            if old.windows == layout.windows && old.icons == layout.icons {
                return false;
            }
        }
        self.layouts.insert(layout.key.clone(), layout);
        self.flush();
        true
    }

    pub fn set_pinned(&mut self, key: &str, pinned: bool) -> bool {
        match self.layouts.get_mut(key) {
            Some(layout) => {
                layout.pinned = pinned;
                self.flush();
                true
            }
            None => false,
        }
    }

    pub fn remove(&mut self, key: &str) -> bool {
        let removed = self.layouts.remove(key).is_some();
        if removed {
            self.flush();
        }
        removed
    }

    fn flush(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match serde_json::to_string_pretty(&self.layouts) {
            Ok(text) => {
                // Write then rename, so a crash never leaves a half-written file.
                let tmp = self.path.with_extension("json.tmp");
                if std::fs::write(&tmp, text).is_ok() {
                    let _ = std::fs::rename(&tmp, &self.path);
                }
            }
            Err(e) => log::error!("could not save layouts: {e}"),
        }
    }
}
