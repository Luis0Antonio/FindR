mod commands;
mod convert;
mod icon_protocol;
mod index;
mod links;
mod macos;
mod search;
mod settings;
mod watcher;

use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, EventTarget, Manager, WebviewWindow, WindowEvent};

static LAST_FOCUSED: Mutex<String> = Mutex::new(String::new());

/// The window the user last worked in, which receives menu commands and opened links.
pub fn focused_window(app: &AppHandle) -> Option<WebviewWindow> {
    let label = LAST_FOCUSED.lock().unwrap().clone();
    app.get_webview_window(&label).or_else(|| app.get_webview_window("main")).or_else(|| app.webview_windows().into_values().next())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(links::Links::default())
        .register_asynchronous_uri_scheme_protocol("icon", |_ctx, request, responder| {
            let path = request.uri().path().to_string();
            tauri::async_runtime::spawn_blocking(move || responder.respond(icon_protocol::handle(&path)));
        })
        .setup(|app| {
            macos::quicklook::init(app.handle());
            app.set_menu(app_menu(app.handle())?)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id() == "settings" {
                if let Some(window) = focused_window(app) {
                    let _ = app.emit_to(EventTarget::webview_window(window.label()), "menu://settings", ());
                }
            }
        })
        .on_window_event(|window, event| match event {
            WindowEvent::Focused(true) => *LAST_FOCUSED.lock().unwrap() = window.label().to_string(),
            WindowEvent::Destroyed => watcher::unwatch_window(window.label()),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_dir,
            commands::open_path,
            commands::home_dir,
            links::startup_target,
            commands::open_privacy_settings,
            commands::create_folder,
            commands::rename,
            commands::list_volumes,
            commands::paths_exist,
            commands::move_items,
            commands::volume_of,
            commands::has_special_icon,
            commands::empty_trash,
            commands::new_window,
            commands::close_window,
            commands::reveal_in_finder,
            commands::drag_out,
            commands::convert_file,
            commands::move_to_trash,
            commands::clipboard_write_files,
            commands::clipboard_read_files,
            commands::quicklook_show,
            commands::quicklook_update,
            commands::quicklook_close,
            search::start_search,
            search::cancel_search,
            index::configure_index,
            index::index_status,
            watcher::watch_dir,
            watcher::unwatch_dir,
            settings::load_settings,
            settings::save_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Opened { urls } = event {
                links::handle_opened(app, &urls);
            }
        });
}

// Cmd+W closes the active pane, so the native Close Window item must not claim it.
fn app_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::default(app)?;
    if let Some(MenuItemKind::Submenu(app_submenu)) = menu.items()?.first() {
        let settings = MenuItem::with_id(app, "settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
        app_submenu.insert_items(&[&settings, &PredefinedMenuItem::separator(app)?], 2)?;
    }
    for item in menu.items()? {
        let MenuItemKind::Submenu(sub) = &item else { continue };
        if sub.text()? == "File" {
            menu.remove(&item)?;
            continue;
        }
        for child in sub.items()? {
            if let MenuItemKind::Predefined(p) = &child {
                if p.text()? == "Close Window" {
                    sub.remove(&child)?;
                }
            }
        }
    }
    Ok(menu)
}
