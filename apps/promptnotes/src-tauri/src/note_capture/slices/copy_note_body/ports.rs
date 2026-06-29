/// OS clipboard write failure surface. Variant set is intentionally small —
/// spec.md#io-errors pins it to `Unavailable | Io(String)`. Expansion (when
/// the phase 7 Tauri adapter is wired) requires updating spec + tests together.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClipboardErrorKind {
    /// OS clipboard daemon unavailable (headless / locked session etc.).
    #[error("OS clipboard unavailable")]
    Unavailable,
    /// Generic write failure surfaced as a string from the adapter.
    #[error("clipboard io: {0}")]
    Io(String),
}

/// Output port for writing plain text to the OS clipboard. The slice's only
/// observable side effect goes through this trait — `tests.rs` substitutes a
/// spy implementation to verify I-CNB1 (body-only), I-CNB3 (order), and the
/// no-event invariant I-CNB4 (no `EventBus` parameter on this trait).
pub trait ClipboardService {
    fn write_text(&self, text: &str) -> Result<(), ClipboardErrorKind>;
}

impl<T: ClipboardService + ?Sized> ClipboardService for std::rc::Rc<T> {
    fn write_text(&self, text: &str) -> Result<(), ClipboardErrorKind> {
        (**self).write_text(text)
    }
}
