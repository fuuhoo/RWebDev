mod dav;
mod preview;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(dav::DavState::default())
        .invoke_handler(tauri::generate_handler![
            dav::test_connection,
            dav::list_dir,
            dav::create_folder,
            dav::delete_entry,
            dav::rename_entry,
            dav::download_file,
            dav::upload_file,
            dav::upload_local_file,
            dav::set_config,
            dav::get_config,
            dav::list_connections,
            dav::get_connection,
            dav::save_connection,
            dav::delete_connection,
            dav::disconnect,
            dav::connect,
            preview::preview_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
