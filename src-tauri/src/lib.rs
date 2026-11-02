mod engine;
mod model;
mod platform;
mod settings;
mod store;

use engine::{Engine, RestoreReport, Status};
use serde::{Deserialize, Serialize};
use settings::Settings;
use std::sync::Arc;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, State, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

/// Passed by the login item / startup entry so Mooring starts quietly in the tray.
const AUTOSTART_ARG: &str = "--autostart";

#[cfg(target_os = "macos")]
const LOGIN_LABEL: &str = "Open at Login";
#[cfg(not(target_os = "macos"))]
const LOGIN_LABEL: &str = "Run at Startup";

/// Settings as the UI sees them: the saved settings plus the OS login-item state.
#[derive(Serialize, Deserialize)]
struct SettingsView {
    #[serde(flatten)]
    settings: Settings,
    launch_at_login: bool,
}

/// Tray check items, kept so the settings window and the tray stay in sync.
struct TrayChecks {
    auto_restore: CheckMenuItem<Wry>,
    launch_at_login: CheckMenuItem<Wry>,
}

fn settings_view(app: &AppHandle, engine: &Engine) -> SettingsView {
    SettingsView {
        settings: engine.settings(),
        launch_at_login: app.autolaunch().is_enabled().unwrap_or(false),
    }
}

fn sync_tray(app: &AppHandle) {
    let (Some(checks), Some(engine)) = (
        app.try_state::<TrayChecks>(),
        app.try_state::<Arc<Engine>>(),
    ) else {
        return;
    };
    let view = settings_view(app, &engine);
    let _ = checks.auto_restore.set_checked(view.settings.auto_restore);
    let _ = checks.launch_at_login.set_checked(view.launch_at_login);
}

fn set_launch_at_login(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(app: AppHandle, engine: State<'_, Arc<Engine>>) -> SettingsView {
    settings_view(&app, &engine)
}

#[tauri::command]
fn update_settings(
    app: AppHandle,
    engine: State<'_, Arc<Engine>>,
    view: SettingsView,
) -> Result<SettingsView, String> {
    if app.autolaunch().is_enabled().unwrap_or(false) != view.launch_at_login {
        set_launch_at_login(&app, view.launch_at_login)?;
    }
    engine.set_settings(view.settings)?;
    sync_tray(&app);
    Ok(settings_view(&app, &engine))
}

#[tauri::command]
fn get_status(engine: State<'_, Arc<Engine>>) -> Status {
    engine.status()
}

#[tauri::command]
async fn save_now(engine: State<'_, Arc<Engine>>) -> Result<usize, String> {
    let engine = Arc::clone(&engine);
    tauri::async_runtime::spawn_blocking(move || engine.snapshot(true))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn restore_now(engine: State<'_, Arc<Engine>>) -> Result<RestoreReport, String> {
    let engine = Arc::clone(&engine);
    tauri::async_runtime::spawn_blocking(move || engine.restore_now())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn set_pinned(engine: State<'_, Arc<Engine>>, key: String, pinned: bool) -> bool {
    engine.set_pinned(&key, pinned)
}

#[tauri::command]
fn forget_layout(engine: State<'_, Arc<Engine>>, key: String) -> bool {
    engine.forget(&key)
}

#[tauri::command]
fn request_permission() {
    platform::request_permission();
}

fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let engine = app.state::<Arc<Engine>>();
    let view = settings_view(app, &engine);

    let restore = MenuItem::with_id(app, "restore", "Restore Windows Now", true, None::<&str>)?;
    let save = MenuItem::with_id(app, "save", "Save Current Layout", true, None::<&str>)?;
    let auto_restore = CheckMenuItem::with_id(
        app,
        "auto",
        "Auto-Restore",
        true,
        view.settings.auto_restore,
        None::<&str>,
    )?;
    let launch_at_login = CheckMenuItem::with_id(
        app,
        "login",
        LOGIN_LABEL,
        true,
        view.launch_at_login,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Mooring", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &restore,
            &save,
            &PredefinedMenuItem::separator(app)?,
            &auto_restore,
            &launch_at_login,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &quit,
        ],
    )?;
    app.manage(TrayChecks {
        auto_restore,
        launch_at_login,
    });

    let builder = TrayIconBuilder::with_id("main")
        .tooltip("Mooring")
        .menu(&menu);
    #[cfg(target_os = "macos")]
    let builder = builder
        .icon(tauri::image::Image::from_bytes(include_bytes!(
            "../icons/tray-template.png"
        ))?)
        .icon_as_template(true);
    #[cfg(not(target_os = "macos"))]
    let builder = match app.default_window_icon() {
        Some(icon) => builder.icon(icon.clone()),
        None => builder,
    };

    builder
        .on_menu_event(|app, event| {
            let engine = Arc::clone(&app.state::<Arc<Engine>>());
            match event.id.as_ref() {
                "restore" => {
                    std::thread::spawn(move || {
                        let _ = engine.restore_now();
                    });
                }
                "save" => {
                    std::thread::spawn(move || {
                        let _ = engine.snapshot(true);
                    });
                }
                "auto" => {
                    let mut settings = engine.settings();
                    settings.auto_restore = !settings.auto_restore;
                    let _ = engine.set_settings(settings);
                    sync_tray(app);
                }
                "login" => {
                    let enabled = app.autolaunch().is_enabled().unwrap_or(false);
                    if let Err(e) = set_launch_at_login(app, !enabled) {
                        log::error!("could not change login item: {e}");
                    }
                    sync_tray(app);
                }
                "settings" => show_settings(app),
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch (e.g. the desktop icon) just opens Settings.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_settings(app)
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_ARG]),
        ))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .build(),
        )
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let dir = app.path().app_data_dir()?;
            let (settings, first_run) = Settings::load(&dir);
            if first_run {
                // Run at startup is on by default; it can be turned off in the tray or Settings.
                if let Err(e) = set_launch_at_login(app.handle(), true) {
                    log::warn!("could not enable launch at login: {e}");
                }
                let _ = settings.save(&dir);
            }

            let engine = Engine::new(dir, settings);
            engine.start();
            app.manage(engine);
            build_tray(app.handle())?;

            let quiet_start = std::env::args().any(|a| a == AUTOSTART_ARG);
            let needs_permission = !platform::has_permission();
            if needs_permission {
                platform::request_permission();
            }
            if first_run || needs_permission || !quiet_start {
                show_settings(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing Settings hides it; Mooring keeps running in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            get_status,
            save_now,
            restore_now,
            set_pinned,
            forget_layout,
            request_permission,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mooring");
}
