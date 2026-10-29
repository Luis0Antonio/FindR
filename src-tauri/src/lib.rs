mod commands;
mod icon_protocol;
mod macos;
mod settings;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol("icon", |_ctx, request, responder| {
            let path = request.uri().path().to_string();
            tauri::async_runtime::spawn_blocking(move || responder.respond(icon_protocol::handle(&path)));
        })
        .setup(|app| {
            macos::quicklook::init(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_dir,
            commands::open_path,
            commands::home_dir,
            commands::startup_dir,
            commands::open_privacy_settings,
            commands::debug_log,
            commands::quicklook_show,
            commands::quicklook_update,
            commands::quicklook_close,
            settings::load_settings,
            settings::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
