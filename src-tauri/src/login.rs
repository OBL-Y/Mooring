//! Starting Mooring when you sign in.
//!
//! Windows uses the autostart plugin (a per-user `Run` registry value).
//! macOS registers the launch agent bundled in the app
//! (`Contents/Library/LaunchAgents`) through `SMAppService`, so it shows up under
//! Login Items & Extensions and macOS can tell us when it needs approval.

use tauri::AppHandle;

/// Passed by the login item / startup entry so Mooring starts quietly in the tray.
pub const AUTOSTART_ARG: &str = "--autostart";

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use objc2_foundation::NSString;
    use objc2_service_management::{SMAppService, SMAppServiceStatus};

    const PLIST: &str = "io.github.obl-y.mooring.login.plist";

    fn service() -> objc2::rc::Retained<SMAppService> {
        unsafe { SMAppService::agentServiceWithPlistName(&NSString::from_str(PLIST)) }
    }

    fn status() -> SMAppServiceStatus {
        unsafe { service().status() }
    }

    pub fn is_enabled(_app: &AppHandle) -> bool {
        status() == SMAppServiceStatus::Enabled
    }

    pub fn needs_approval(_app: &AppHandle) -> bool {
        status() == SMAppServiceStatus::RequiresApproval
    }

    pub fn set(app: &AppHandle, enabled: bool) -> Result<(), String> {
        let service = service();
        let result = unsafe {
            if enabled {
                service.registerAndReturnError()
            } else {
                service.unregisterAndReturnError()
            }
        };
        if let Err(e) = result {
            // Unregistering something that isn't registered isn't a problem.
            if enabled || unsafe { service.status() } != SMAppServiceStatus::NotRegistered {
                return Err(e.localizedDescription().to_string());
            }
        }
        if enabled && needs_approval(app) {
            // Turned off in System Settings: send people there to allow it.
            open_settings();
        }
        Ok(())
    }

    pub fn open_settings() {
        unsafe { SMAppService::openSystemSettingsLoginItems() };
    }

    /// Versions up to 0.1.0-beta.4 wrote `~/Library/LaunchAgents/Mooring.plist`.
    /// Replace it with the bundled agent, keeping the person's choice.
    pub fn migrate(app: &AppHandle) {
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };
        let legacy = std::path::Path::new(&home).join("Library/LaunchAgents/Mooring.plist");
        if !legacy.exists() {
            return;
        }
        log::info!("replacing old login item with the bundled one");
        // Keep the old item unless the new one registered, so Mooring still starts at login.
        if let Err(e) = set(app, true) {
            log::warn!("could not register login item, keeping the old one: {e}");
            return;
        }
        if let Err(e) = std::fs::remove_file(&legacy) {
            log::warn!("could not remove old login item: {e}");
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
