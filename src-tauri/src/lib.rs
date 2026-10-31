mod commands;
mod convert;
mod icon_protocol;
mod macos;
mod search;
mod settings;
mod watcher;

use tauri::menu::{Menu, MenuItemKind};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol("icon", |_ctx, request, responder| {
            let path = request.uri().path().to_string();
            tauri::async_runtime::spawn_blocking(move || responder.respond(icon_protocol::handle(&path)));
        })
        .setup(|app| {
            macos::quicklook::init(app.handle());
            app.set_menu(menu_without_close_window(app.handle())?)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_dir,
            commands::open_path,
            commands::home_dir,
            commands::startup_dir,
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Cmd+W closes the active pane, so the native Close Window item must not claim it.
fn menu_without_close_window(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::default(app)?;
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
