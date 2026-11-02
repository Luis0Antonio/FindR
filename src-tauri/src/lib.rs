mod commands;
mod convert;
mod icon_protocol;
mod links;
mod macos;
mod search;
mod settings;
mod watcher;

use tauri::menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem};
use tauri::Emitter;

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
                let _ = app.emit("menu://settings", ());
            }
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
