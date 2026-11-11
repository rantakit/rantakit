#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod camera;
#[cfg(target_os = "android")]
mod apk_update;
mod commands;
mod error;
#[cfg(target_os = "macos")]
mod menu;
mod notice_words;
#[cfg(target_os = "android")]
mod share_in;
#[cfg(target_os = "android")]
mod task_notice;

use rantakit_tasks::{History, PreparedPages, Presets, RecentLimits, RecentTools, SettingsStore};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            app.manage(History::new(data.join("history.json")));
            app.manage(RecentLimits::limits(data.join("recent-limits.json")));
            app.manage(RecentTools::tools(data.join("recent-tools.json")));
            app.manage(SettingsStore::new(data.join("settings.json")));
            app.manage(Presets::new(data.join("presets.json")));
            // Scan Dokumen's pages made ahead; nothing from an earlier run is kept.
            app.manage(PreparedPages::new(app.path().app_cache_dir()?.join("prepared-pages")));
            app.manage(commands::task::RunningTasks::default());
            app.manage(commands::pages::OpenPdfs::default());
            app.manage(commands::watermark::PreviewBase::default());
            app.manage(commands::update::PendingUpdate::default());
            app.manage(commands::language::Language::default());
            // Copies left by a Task that Android cut short.
            #[cfg(target_os = "android")]
            android::clear_workspaces(app.handle());
            // Photos taken for a Scan Dokumen Task that was cut short.
            #[cfg(target_os = "android")]
            camera::clear(app.handle());
            #[cfg(any(windows, target_os = "linux"))]
            {
                let window = main_window(app)?;
                #[cfg(windows)]
                round_corners(&window);
            }
            Ok(())
        });

    // Chosen files and saved Results go through `content://` addresses on Android.
    #[cfg(target_os = "android")]
    let builder = builder.plugin(tauri_plugin_android_fs::init());

    // Tasks keep running in a foreground service when the user switches apps.
    #[cfg(target_os = "android")]
    let builder = builder.plugin(task_notice::init());

    // Photos and PDFs shared from other apps.
    #[cfg(target_os = "android")]
    let builder = builder.plugin(share_in::init());

    // "Versi baru" for the APK from GitHub.
    #[cfg(target_os = "android")]
    let builder = builder.plugin(apk_update::init());

    // Scan Dokumen's camera.
    #[cfg(target_os = "android")]
    let builder = builder.plugin(camera::init());

    // Installs new versions on desktop; the APK only learns of them (apk_update.rs).
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());

    // Only macOS gets a native menu bar; Windows and Linux use the in-app menu.
    #[cfg(target_os = "macos")]
    let builder = builder.menu(menu::build).on_menu_event(menu::handle_event);

    builder
        .invoke_handler(tauri::generate_handler![
            commands::files::file_info,
            commands::files::pick_files,
            commands::files::file_names,
            commands::files::take_shared_files,
            commands::files::thumbnail,
            commands::pages::page_thumbnail,
            commands::photos::find_paper,
            commands::photos::edited_preview,
            commands::photos::prepare_page,
            commands::watermark::watermark_preview,
            commands::camera::camera_permission,
            commands::camera::ask_camera_permission,
            commands::camera::open_app_settings,
            commands::camera::take_photos,
            commands::camera::discard_photos,
            commands::files::open_path,
            commands::files::can_save_results,
            commands::files::ask_to_save_results,
            commands::share::share_files,
            commands::language::set_language,
            commands::tools::list_tools,
            commands::tools::match_files,
            commands::task::start_task,
            commands::task::cancel_task,
            commands::task::notification_permission,
            commands::task::ask_notification_permission,
            commands::history::list_history,
            commands::history::clear_history,
            commands::recent::recent_limits,
            commands::recent::home_tools,
            commands::settings::get_settings,
            commands::settings::set_keep_history,
            commands::settings::set_results_folder,
            commands::settings::set_check_updates,
            commands::update::can_check_updates,
            commands::update::check_update,
            commands::update::install_update,
            commands::presets::list_presets,
            commands::presets::save_preset,
            commands::presets::rename_preset,
            commands::presets::remove_preset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Opens the main window on Windows and Linux. Their config sets `create:
/// false` so it is made here, with clipboard access: without it, Paste in the
/// title bar menu makes WebView2 ask for permission and fails on WebKitGTK.
/// macOS has its native Edit menu instead. The configured size shrinks to fit
/// small screens.
#[cfg(any(windows, target_os = "linux"))]
fn main_window(app: &tauri::App) -> tauri::Result<tauri::WebviewWindow> {
    let mut config = app
        .config()
        .app
        .windows
        .first()
        .cloned()
        .unwrap_or_default();
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let screen = monitor
            .work_area()
            .size
            .to_logical::<f64>(monitor.scale_factor());
        config.width = fit(config.width, screen.width, config.min_width);
        config.height = fit(config.height, screen.height, config.min_height);
    }
    tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?
        .enable_clipboard_access()
        .build()
}

/// The configured window size along one side, kept within 90% of the screen's
/// work area, so a 1366×768 laptop gets a window that fits with the taskbar,
/// but never below the minimum size.
#[cfg(any(windows, target_os = "linux"))]
fn fit(wanted: f64, screen: f64, min: Option<f64>) -> f64 {
    wanted.min((screen * 0.9).floor()).max(min.unwrap_or(0.0))
}

/// Asks Windows 11 to round the undecorated window like a native one (about
/// 8 px). Windows 10 ignores it and keeps square corners.
#[cfg(windows)]
fn round_corners(window: &tauri::WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{
        DWM_WINDOW_CORNER_PREFERENCE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        DwmSetWindowAttribute,
    };
    let Ok(hwnd) = window.hwnd() else { return };
    let preference = DWMWCP_ROUND;
    // Cosmetic only: a failure leaves the system default corners.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&raw const preference).cast(),
            size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        )
    };
}
