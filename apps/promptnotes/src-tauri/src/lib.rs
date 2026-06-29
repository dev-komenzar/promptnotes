pub mod note_capture;
pub mod note_feed;
pub mod update_distribution;
pub mod user_preferences;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(note_feed::shared::adapters::InMemoryNoteFeedState::new())
        .manage(note_capture::shared::adapters::undo_stack::InMemoryUndoStack::new())
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
        // S13 (.ori/domain/validation.md#s13-quit-flush) の連続 Flush は
        // frontend (PageMain.svelte) が CloseRequested を JS で intercept し、
        // pendingFlushRegistry を順次 await → window.destroy() する案 1 で実装。
        // Rust 側に on_window_event hook を置かないのは flush-note slice の
        // C-FL11 (use case は 1 件 stateless) を保ったまま順序保証を
        // composition root に寄せるため。
        // 詳細: .ori/slices/flush-note/spec.md#impl-quit-orchestration / ori-73q
        .invoke_handler(tauri::generate_handler![
            note_capture::slices::create_note::commands::create_note,
            note_capture::slices::auto_save_note::commands::auto_save_note,
            note_capture::slices::copy_note_body::commands::copy_note_body,
            note_capture::slices::flush_note::commands::flush_note,
            note_capture::slices::assign_tag::commands::assign_tag,
            note_capture::slices::remove_tag::commands::remove_tag,
            note_capture::slices::delete_note::commands::delete_note,
            note_capture::slices::restore_deleted_note::commands::restore_deleted_note,
            note_feed::slices::update_feed_filter::commands::update_feed_filter,
            note_feed::slices::change_sort_order::commands::change_sort_order,
            note_feed::slices::list_feed::commands::list_notes,
            user_preferences::slices::load_settings::commands::load_settings,
            user_preferences::slices::update_settings::commands::update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
