import { invoke } from '@tauri-apps/api/core';

export type CopyNoteBodyError =
	| { kind: 'note_not_found'; id: string }
	| { kind: 'clipboard_error'; variant: 'unavailable' | 'io'; reason: string };

/**
 * Invoke the `copy_note_body` Tauri command. Resolves with `void` on success,
 * rejects with a `CopyNoteBodyError` payload mirroring spec.md#io-errors.
 *
 * Mirrors the slice port `ClipboardService::write_text` semantics: the OS
 * clipboard receives `Note::body_for_clipboard()` only (I-CNB1).
 */
export async function copyNoteBody(noteId: string): Promise<void> {
	await invoke<void>('copy_note_body', { noteId });
}
