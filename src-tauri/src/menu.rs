//! Native menu bar, macOS only. Windows and Linux have keyboard shortcuts and
//! the hamburger menu in the title bar instead, see `src/lib/appCommands.ts`.
//!
//! Links are opened here; the other items are forwarded to the main window as
//! a `menu` event carrying the item id. The labels follow the UI language:
//! the main window calls the `set_language` command on start and on every change.

use tauri::menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_opener::OpenerExt;

pub const OPEN_FILES: &str = "open_files";
pub const GO_HOME: &str = "go_home";
pub const GO_TOOLS: &str = "go_tools";
pub const GO_HISTORY: &str = "go_history";
pub const GO_BACK: &str = "go_back";
pub const GO_FORWARD: &str = "go_forward";
pub const TOGGLE_SIDEBAR: &str = "toggle_sidebar";
pub const OPEN_SOURCE: &str = "open_source";
pub const OPEN_LICENSE: &str = "open_license";
pub const OPEN_SETTINGS: &str = "open_settings";

/// Event the main window listens to for the items it handles.
const MENU_EVENT: &str = "menu";
const SOURCE_URL: &str = "https://github.com/rantakit/rantakit";
const LICENSE_URL: &str = "https://www.gnu.org/licenses/agpl-3.0.html";

/// Every label in the menu bar, in one language.
struct Labels {
    about: &'static str,
    tagline: &'static str,
    settings: &'static str,
    hide: &'static str,
    hide_others: &'static str,
    show_all: &'static str,
    quit: &'static str,
    file: &'static str,
    open_files: &'static str,
    edit: &'static str,
    cut: &'static str,
    copy: &'static str,
    paste: &'static str,
    select_all: &'static str,
    view: &'static str,
    home: &'static str,
    tools: &'static str,
    history: &'static str,
    back: &'static str,
    forward: &'static str,
    toggle_sidebar: &'static str,
    window: &'static str,
    minimize: &'static str,
    maximize: &'static str,
    close_window: &'static str,
    help: &'static str,
    source: &'static str,
    license: &'static str,
}

const ID: Labels = Labels {
    about: "Tentang Rantakit",
    tagline: "File tetap di perangkatmu.",
    settings: "Pengaturan…",
    hide: "Sembunyikan Rantakit",
    hide_others: "Sembunyikan lainnya",
    show_all: "Tampilkan semua",
    quit: "Keluar dari Rantakit",
    file: "File",
    open_files: "Buka…",
    edit: "Edit",
    cut: "Potong",
    copy: "Salin",
    paste: "Tempel",
    select_all: "Pilih semua",
    view: "Tampilan",
    home: "Beranda",
    tools: "Peralatan",
    history: "Riwayat",
    back: "Kembali",
    forward: "Maju",
    toggle_sidebar: "Tampilkan/Sembunyikan navigasi",
    window: "Jendela",
    minimize: "Perkecil",
    maximize: "Perbesar",
    close_window: "Tutup jendela",
    help: "Bantuan",
    source: "Kode sumber",
    license: "Lisensi",
};

const EN: Labels = Labels {
    about: "About Rantakit",
    tagline: "Your files stay on your device.",
    settings: "Settings…",
    hide: "Hide Rantakit",
    hide_others: "Hide Others",
    show_all: "Show All",
    quit: "Quit Rantakit",
    file: "File",
    open_files: "Open…",
    edit: "Edit",
    cut: "Cut",
    copy: "Copy",
    paste: "Paste",
    select_all: "Select All",
    view: "View",
    home: "Home",
    tools: "Tools",
    history: "History",
    back: "Back",
    forward: "Forward",
    toggle_sidebar: "Show/Hide Navigation",
    window: "Window",
    minimize: "Minimize",
    maximize: "Zoom",
    close_window: "Close Window",
    help: "Help",
    source: "Source Code",
    license: "License",
};

fn labels(language: &str) -> &'static Labels {
    if language == "en" { &EN } else { &ID }
}

/// The menu bar at startup, before the main window says which language it shows.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    build_in(app, &ID)
}

/// Rebuilds the menu bar in `language` ("id" or "en").
pub fn set_language<R: Runtime>(app: &AppHandle<R>, language: &str) -> tauri::Result<()> {
    app.set_menu(build_in(app, labels(language))?).map(|_| ())
}

fn build_in<R: Runtime>(app: &AppHandle<R>, l: &Labels) -> tauri::Result<Menu<R>> {
    let about = PredefinedMenuItem::about(
        app,
        Some(l.about),
        Some(AboutMetadata {
            name: Some("Rantakit".into()),
            version: Some(app.package_info().version.to_string()),
            comments: Some(l.tagline.into()),
            license: Some("AGPL-3.0-or-later".into()),
            ..Default::default()
        }),
    )?;

    let app_menu = Submenu::with_items(
        app,
        "Rantakit",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, OPEN_SETTINGS, l.settings, true, Some("CmdOrCtrl+,"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, Some(l.hide))?,
            &PredefinedMenuItem::hide_others(app, Some(l.hide_others))?,
            &PredefinedMenuItem::show_all(app, Some(l.show_all))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some(l.quit))?,
        ],
    )?;

    let file = Submenu::with_items(
        app,
        l.file,
        true,
        &[&MenuItem::with_id(
            app,
            OPEN_FILES,
            l.open_files,
            true,
            Some("CmdOrCtrl+O"),
        )?],
    )?;

    // Without these, copy and paste shortcuts don't work in text fields on macOS.
    let edit = Submenu::with_items(
        app,
        l.edit,
        true,
        &[
            &PredefinedMenuItem::cut(app, Some(l.cut))?,
            &PredefinedMenuItem::copy(app, Some(l.copy))?,
            &PredefinedMenuItem::paste(app, Some(l.paste))?,
            &PredefinedMenuItem::select_all(app, Some(l.select_all))?,
        ],
    )?;

    let view = Submenu::with_items(
        app,
        l.view,
        true,
        &[
            &MenuItem::with_id(app, GO_HOME, l.home, true, Some("CmdOrCtrl+1"))?,
            &MenuItem::with_id(app, GO_TOOLS, l.tools, true, Some("CmdOrCtrl+2"))?,
            &MenuItem::with_id(app, GO_HISTORY, l.history, true, Some("CmdOrCtrl+3"))?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, GO_BACK, l.back, true, Some("CmdOrCtrl+["))?,
            &MenuItem::with_id(app, GO_FORWARD, l.forward, true, Some("CmdOrCtrl+]"))?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, TOGGLE_SIDEBAR, l.toggle_sidebar, true, Some("CmdOrCtrl+B"))?,
        ],
    )?;

    // Gives Cmd+M and Cmd+W their usual meaning.
    let window = Submenu::with_items(
        app,
        l.window,
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some(l.minimize))?,
            &PredefinedMenuItem::maximize(app, Some(l.maximize))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, Some(l.close_window))?,
        ],
    )?;

    let help = Submenu::with_items(
        app,
        l.help,
        true,
        &[
            &MenuItem::with_id(app, OPEN_SOURCE, l.source, true, None::<&str>)?,
            &MenuItem::with_id(app, OPEN_LICENSE, l.license, true, None::<&str>)?,
        ],
    )?;

    Menu::with_items(app, &[&app_menu, &file, &edit, &view, &window, &help])
}

pub fn handle_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        OPEN_SOURCE => open_url(app, SOURCE_URL),
        OPEN_LICENSE => open_url(app, LICENSE_URL),
        id @ (OPEN_FILES | GO_HOME | GO_TOOLS | GO_HISTORY | GO_BACK | GO_FORWARD | TOGGLE_SIDEBAR
        | OPEN_SETTINGS) => {
            if let Err(err) = app.emit_to("main", MENU_EVENT, id) {
                eprintln!("failed to forward menu item {id}: {err}");
            }
        }
        _ => {}
    }
}

fn open_url<R: Runtime>(app: &AppHandle<R>, url: &str) {
    if let Err(err) = app.opener().open_url(url, None::<&str>) {
        eprintln!("failed to open {url}: {err}");
    }
}
