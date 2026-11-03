//! "Export Diagnostics": a zip of everything useful for a bug report.
//!
//! It holds the logs, saved layouts and settings, and `system.json` with the
//! Mooring version, OS, displays and current status. It's saved to Downloads
//! and shown in Finder / Explorer, ready to attach to an issue.

use crate::engine::{now_secs, Engine};
use crate::{login, platform};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Builds the zip and returns where it was saved.
pub fn export(app: &AppHandle, engine: &Engine) -> Result<PathBuf, String> {
    let path = app.path();
    let out_dir = path
        .download_dir()
        .or_else(|_| path.desktop_dir())
        .map_err(|e| e.to_string())?;
    let out = out_dir.join(format!("Mooring diagnostics {}.zip", timestamp()));
    let data_dir = path.app_data_dir().map_err(|e| e.to_string())?;
    let log_dir = path.app_log_dir().map_err(|e| e.to_string())?;

    let file = File::create(&out).map_err(|e| format!("Couldn't create {}: {e}", out.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    let system = serde_json::to_string_pretty(&system_info(app, engine)).unwrap_or_default();
    add_bytes(&mut zip, "system.json", system.as_bytes(), options)?;
    for name in ["settings.json", "layouts.json"] {
        add_file(&mut zip, name, &data_dir.join(name), options)?;
    }
    if let Ok(entries) = std::fs::read_dir(&log_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "log") {
                let name = format!("logs/{}", entry.file_name().to_string_lossy());
                add_file(&mut zip, &name, &p, options)?;
            }
        }
    }
    zip.finish().map_err(|e| e.to_string())?;

    log::info!("exported diagnostics to {}", out.display());
    reveal(&out);
    Ok(out)
}

fn system_info(app: &AppHandle, engine: &Engine) -> serde_json::Value {
    json!({
        "mooring_version": app.package_info().version.to_string(),
        "platform": platform::NAME,
        "os_version": os_version(),
        "arch": std::env::consts::ARCH,
        "exported_at": now_secs(),
        "launch_at_login": login::is_enabled(app),
        "login_needs_approval": login::needs_approval(app),
        "displays": platform::displays(),
        "open_windows": platform::windows().len(),
        "status": engine.status(),
    })
}

fn add_bytes(
    zip: &mut ZipWriter<File>,
    name: &str,
    bytes: &[u8],
    options: SimpleFileOptions,
) -> Result<(), String> {
    zip.start_file(name, options).map_err(|e| e.to_string())?;
    zip.write_all(bytes).map_err(|e| e.to_string())
}

/// Adds a file if it exists; a missing file (say, no layouts yet) is skipped.
fn add_file(
    zip: &mut ZipWriter<File>,
    name: &str,
    path: &Path,
    options: SimpleFileOptions,
) -> Result<(), String> {
    match std::fs::read(path) {
        Ok(bytes) => add_bytes(zip, name, &bytes, options),
        Err(_) => Ok(()),
    }
}

/// Local date and time for the file name, e.g. `2026-11-03 1142`.
fn timestamp() -> String {
    #[cfg(target_os = "macos")]
    let output = std::process::Command::new("date")
        .arg("+%Y-%m-%d %H%M")
        .output();
    #[cfg(windows)]
    let output = hidden_command("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-Date -Format 'yyyy-MM-dd HHmm'",
        ])
        .output();
    #[cfg(not(any(target_os = "macos", windows)))]
    let output = std::process::Command::new("date")
        .arg("+%Y-%m-%d %H%M")
        .output();
    output
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| now_secs().to_string())
}

fn os_version() -> String {
    #[cfg(target_os = "macos")]
    let output = std::process::Command::new("sw_vers").output();
    #[cfg(windows)]
    let output = hidden_command("cmd").args(["/C", "ver"]).output();
    #[cfg(not(any(target_os = "macos", windows)))]
    let output = std::process::Command::new("uname").arg("-a").output();
    output
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

/// Shows the file selected in Finder / Explorer.
fn reveal(path: &Path) {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn();
    #[cfg(windows)]
    let result = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
    #[cfg(not(any(target_os = "macos", windows)))]
    let result = std::process::Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .spawn();
    if let Err(e) = result {
        log::warn!("could not show diagnostics file: {e}");
    }
}

/// A command that doesn't flash a console window.
#[cfg(windows)]
fn hidden_command(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = std::process::Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}
