//! Checks GitHub Releases for a newer Mooring and installs it.
//!
//! Releases publish a signed `latest.json`; the endpoint and public key are in
//! `tauri.conf.json` under `plugins.updater`.

use crate::engine::now_secs;
use serde::Serialize;
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Default)]
struct Inner {
    checking: bool,
    installing: bool,
    available: Option<Update>,
    checked_at: u64,
    error: Option<String>,
    /// The version Settings was last opened for, so each update is announced once.
    announced: Option<String>,
}

/// Update state shared by the tray, the Settings window and the background check.
#[derive(Default)]
pub struct Updates(Mutex<Inner>);

/// What the Settings window shows about updates.
#[derive(Serialize)]
pub struct UpdateView {
    pub current_version: String,
    pub checking: bool,
    pub installing: bool,
    pub available_version: Option<String>,
    pub checked_at: u64,
    pub error: Option<String>,
}

impl Updates {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn view(&self, app: &AppHandle) -> UpdateView {
        let inner = self.lock();
        UpdateView {
            current_version: app.package_info().version.to_string(),
            checking: inner.checking,
            installing: inner.installing,
            available_version: inner.available.as_ref().map(|u| u.version.clone()),
            checked_at: inner.checked_at,
            error: inner.error.clone(),
        }
    }

    pub fn available_version(&self) -> Option<String> {
        self.lock().available.as_ref().map(|u| u.version.clone())
    }

    /// True the first time it's called for a given version.
    pub fn first_announcement(&self, version: &str) -> bool {
        let mut inner = self.lock();
        if inner.announced.as_deref() == Some(version) {
            return false;
        }
        inner.announced = Some(version.to_string());
        true
    }
}

/// Asks the release server whether a newer version exists.
/// Returns the new version when one is found.
pub async fn check(app: &AppHandle) -> Result<Option<String>, String> {
    let updates = app.state::<Updates>();
    {
        let mut inner = updates.lock();
        if inner.checking || inner.installing {
            return Ok(inner.available.as_ref().map(|u| u.version.clone()));
        }
        inner.checking = true;
    }

    let result = match app.updater() {
        Ok(updater) => updater.check().await.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };

    let mut inner = updates.lock();
    inner.checking = false;
    inner.checked_at = now_secs();
    match result {
        Ok(update) => {
            inner.error = None;
            let version = update.as_ref().map(|u| u.version.clone());
            match &version {
                Some(v) => log::info!("update available: {v}"),
                None => log::info!("no update available"),
            }
            inner.available = update;
            Ok(version)
        }
        Err(e) => {
            log::warn!("update check failed: {e}");
            inner.error = Some("Couldn't reach the update server.".into());
            Err(e)
        }
    }
}

/// Downloads and installs the update found by [`check`], then restarts Mooring.
/// On Windows the installer closes Mooring itself and starts it again when done.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let updates = app.state::<Updates>();
    let update = {
        let mut inner = updates.lock();
        if inner.installing {
            return Ok(());
        }
        let Some(update) = inner.available.clone() else {
            return Err("There's no update to install.".into());
        };
        inner.installing = true;
        update
    };

    log::info!("installing update {}", update.version);
    if let Err(e) = update.download_and_install(|_, _| {}, || {}).await {
        log::error!("update install failed: {e}");
        let mut inner = updates.lock();
        inner.installing = false;
        inner.error = Some("The update couldn't be installed. Please try again.".into());
        return Err(e.to_string());
    }
    app.restart();
}
