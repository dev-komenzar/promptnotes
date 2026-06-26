//! Tauri adapter for the slice port `ClipboardService`.
//!
//! Wraps `tauri-plugin-clipboard-manager` so the use case stays free of
//! Tauri types. Error mapping follows spec.md#io-errors:
//! the plugin's failure surface collapses into `ClipboardErrorKind::Io(_)`;
//! the `Unavailable` variant is reserved for cases the plugin itself reports
//! as no clipboard backend (currently surfaced as an Io message — refine when
//! the plugin grows a distinguished error).

use tauri::{AppHandle, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::note_capture::slices::copy_note_body::{ClipboardErrorKind, ClipboardService};

pub struct TauriClipboardService<'a, R: Runtime> {
    app: &'a AppHandle<R>,
}

impl<'a, R: Runtime> TauriClipboardService<'a, R> {
    pub fn new(app: &'a AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<'a, R: Runtime> ClipboardService for TauriClipboardService<'a, R> {
    fn write_text(&self, text: &str) -> Result<(), ClipboardErrorKind> {
        self.app
            .clipboard()
            .write_text(text.to_string())
            .map_err(|e| ClipboardErrorKind::Io(e.to_string()))
    }
}
