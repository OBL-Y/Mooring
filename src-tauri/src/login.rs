//! Starting Mooring when you sign in.
//!
//! Both systems use the autostart plugin: a per-user `Run` registry value on
//! Windows, and `~/Library/LaunchAgents/Mooring.plist` on macOS. On macOS,
//! `SMAppService` also tells us when Login Items & Extensions has it turned off.
//!
//! (A launch agent bundled in the app and registered through `SMAppService` was
//! tried in 0.1.0-beta.5. macOS didn't start it reliably for an app without an
//! Apple developer team, so it's removed again on start.)

use tauri::AppHandle;

/// Passed by the login item / startup entry so Mooring starts quietly in the tray.
pub const AUTOSTART_ARG: &str = "--autostart";

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use objc2_foundation::{NSString, NSURL};
    use objc2_service_management::{SMAppService, SMAppServiceStatus};
    use tauri::Manager;
    use tauri_plugin_autostart::ManagerExt;

    /// The bundled agent registered by 0.1.0-beta.5.
    const BUNDLED_AGENT: &str = "io.github.obl-y.mooring.login.plist";

    fn plist_path(app: &AppHandle) -> Option<std::path::PathBuf> {
        let home = std::env::var_os("HOME")?;
        let name = format!("{}.plist", app.package_info().name);
        Some(
            std::path::Path::new(&home)
                .join("Library/LaunchAgents")
                .join(name),
        )
    }

    pub fn is_enabled(app: &AppHandle) -> bool {
        app.autolaunch().is_enabled().unwrap_or(false)
    }

    pub fn needs_approval(app: &AppHandle) -> bool {
        if !is_enabled(app) {
            return false;
        }
        let Some(path) = plist_path(app) else {
            return false;
        };
        let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
        unsafe { SMAppService::statusForLegacyURL(&url) == SMAppServiceStatus::RequiresApproval }
    }

    pub fn set(app: &AppHandle, enabled: bool) -> Result<(), String> {
        let autolaunch = app.autolaunch();
        let result = if enabled {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        result.map_err(|e| e.to_string())?;
        if enabled && needs_approval(app) {
            // Turned off in System Settings: send people there to allow it.
            open_settings();
        }
        Ok(())
    }

    pub fn open_settings() {
        unsafe { SMAppService::openSystemSettingsLoginItems() };
    }

    /// Removes the bundled agent that 0.1.0-beta.5 registered, keeping the
    /// person's choice by turning on the usual login item instead.
    pub fn migrate(app: &AppHandle) {
        let agent =
            unsafe { SMAppService::agentServiceWithPlistName(&NSString::from_str(BUNDLED_AGENT)) };
        let status = unsafe { agent.status() };
        if status != SMAppServiceStatus::Enabled && status != SMAppServiceStatus::RequiresApproval {
            return;
        }
        log::info!("removing the bundled login item from an earlier test version");
        if let Err(e) = unsafe { agent.unregisterAndReturnError() } {
            log::warn!(
                "could not remove bundled login item: {}",
                e.localizedDescription()
            );
        }
        if !is_enabled(app) {
            if let Err(e) = set(app, true) {
                log::warn!("could not turn on login item: {e}");
            }
        }
        if let Ok(dir) = app.path().app_data_dir() {
            let _ = std::fs::remove_file(dir.join("login-item"));
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use tauri_plugin_autostart::ManagerExt;

    pub fn is_enabled(app: &AppHandle) -> bool {
        app.autolaunch().is_enabled().unwrap_or(false)
    }

    pub fn needs_approval(_app: &AppHandle) -> bool {
        false
    }

    pub fn set(app: &AppHandle, enabled: bool) -> Result<(), String> {
        let autolaunch = app.autolaunch();
        let result = if enabled {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        result.map_err(|e| e.to_string())
    }

    pub fn open_settings() {}

    pub fn migrate(_app: &AppHandle) {}
}

#[cfg(not(any(target_os = "macos", windows)))]
mod imp {
    use super::*;

    pub fn is_enabled(_app: &AppHandle) -> bool {
        false
    }

    pub fn needs_approval(_app: &AppHandle) -> bool {
        false
    }

    pub fn set(_app: &AppHandle, _enabled: bool) -> Result<(), String> {
        Err("Starting at sign-in isn't supported on this system.".into())
    }

    pub fn open_settings() {}

    pub fn migrate(_app: &AppHandle) {}
}

pub use imp::{is_enabled, migrate, needs_approval, open_settings, set};
