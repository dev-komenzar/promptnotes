pub mod note_capture;
pub mod user_preferences;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // tauri_plugin_updater requires plugins.updater.{endpoints, pubkey} + a signing key.
        // Wire in once release infrastructure is ready.
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            note_capture::slices::create_note::commands::create_note,
            note_capture::slices::auto_save_note::commands::auto_save_note,
            note_capture::slices::flush_note::commands::flush_note,
            note_capture::slices::assign_tag::commands::assign_tag,
            note_capture::slices::remove_tag::commands::remove_tag,
            user_preferences::slices::load_settings::commands::load_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
