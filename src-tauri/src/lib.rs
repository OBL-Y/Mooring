mod engine;
mod login;
mod model;
mod platform;
mod settings;
mod store;
mod updater;

use engine::{Engine, RestoreReport, Status};
use serde::{Deserialize, Serialize};
use settings::Settings;
use std::sync::Arc;
use std::time::Duration;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, State, WindowEvent, Wry};
use updater::{UpdateView, Updates};

use login::AUTOSTART_ARG;

/// How long after start-up to wait before the first update check, so the network is up.
const FIRST_UPDATE_CHECK: Duration = Duration::from_secs(15);
/// How often a long-running Mooring looks for updates again.
const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

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
    /// macOS only: the login item is registered but turned off in System Settings.
    #[serde(default)]
    login_needs_approval: bool,
}

/// Tray items that change, kept so the settings window and the tray stay in sync.
struct TrayChecks {
    auto_restore: CheckMenuItem<Wry>,
    launch_at_login: CheckMenuItem<Wry>,
    update: MenuItem<Wry>,
}

fn settings_view(app: &AppHandle, engine: &Engine) -> SettingsView {
    SettingsView {
        settings: engine.settings(),
        launch_at_login: login::is_enabled(app),
        login_needs_approval: login::needs_approval(app),
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

fn sync_update_item(app: &AppHandle) {
    let Some(checks) = app.try_state::<TrayChecks>() else {
        return;
    };
    let text = match app.state::<Updates>().available_version() {
        Some(version) => format!("Install Update ({version})…"),
        None => "Check for Updates…".to_string(),
    };
    let _ = checks.update.set_text(text);
}

/// Checks for an update and updates the tray. With `announce`, opens Settings
/// the first time a new version is found, so people see it without looking.
async fn run_update_check(app: &AppHandle, announce: bool) {
    let found = updater::check(app).await.ok().flatten();
    sync_update_item(app);
    if let Some(version) = found {
        if announce && app.state::<Updates>().first_announcement(&version) {
            show_settings(app);
        }
    }
}

fn set_launch_at_login(app: &AppHandle, enabled: bool) -> Result<(), String> {
    login::set(app, enabled)
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
    if login::is_enabled(&app) != view.launch_at_login {
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

#[tauri::command]
fn open_login_items() {
    login::open_settings();
}

#[tauri::command]
fn get_update(app: AppHandle, updates: State<'_, Updates>) -> UpdateView {
    updates.view(&app)
}

#[tauri::command]
async fn check_update(app: AppHandle) -> UpdateView {
    run_update_check(&app, false).await;
    app.state::<Updates>().view(&app)
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    updater::install(&app).await
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
    let update = MenuItem::with_id(app, "update", "Check for Updates…", true, None::<&str>)?;
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
            &update,
            &settings,
            &quit,
        ],
    )?;
    app.manage(TrayChecks {
        auto_restore,
        launch_at_login,
        update,
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
                    let enabled = login::is_enabled(app);
                    if let Err(e) = set_launch_at_login(app, !enabled) {
                        log::error!("could not change login item: {e}");
                    }
                    sync_tray(app);
                }
                "update" => {
                    // Settings shows the progress and the result.
                    show_settings(app);
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if app.state::<Updates>().available_version().is_some() {
                            let _ = updater::install(&app).await;
                        } else {
                            run_update_check(&app, false).await;
                        }
                    });
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
    // Must be first: a second launch (e.g. the desktop icon) just opens Settings.
    let builder =
        tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_settings(app)
        }));
    #[cfg(any(target_os = "macos", windows))]
    let builder = builder.plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        Some(vec![AUTOSTART_ARG]),
    ));
    builder
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            } else {
                login::migrate(app.handle());
            }

            let engine = Engine::new(dir, settings);
            engine.start();
            app.manage(Arc::clone(&engine));
            app.manage(Updates::default());
            build_tray(app.handle())?;

            // Look for updates shortly after start-up, then once a day.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(FIRST_UPDATE_CHECK);
                loop {
                    if engine.settings().check_for_updates {
                        tauri::async_runtime::block_on(run_update_check(&handle, true));
                    }
                    std::thread::sleep(UPDATE_CHECK_INTERVAL);
                }
            });

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
            open_login_items,
            get_update,
            check_update,
            install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mooring");
}
