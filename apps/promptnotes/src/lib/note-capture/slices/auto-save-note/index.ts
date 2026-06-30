import { invoke } from '@tauri-apps/api/core';

export type AutoSaveOutcome =
	| { outcome: 'saved'; id: string; updated_at: string }
	| { outcome: 'no_op' };

export type AutoSaveError =
	| { kind: 'note_not_found'; id: string }
	| { kind: 'invalid_body'; reason: string }
	| { kind: 'load_error'; path: string; reason: string }
	| { kind: 'persist_error'; path: string; reason: string };

/**
 * Invoke the `auto_save_note` Tauri command.
 *
 * EDITING-block の debounce 沈静後に呼ばれる trailing-edge 永続化。
 * `new_body` が現行と同値なら NoOp (C-AS3)、それ以外は updated_at を更新して
 * `Saved` を返す。NoteId の parse 失敗は `NoteNotFound` に降格される
 * (spec.md#oq-invalid-note-id)。
 */
export async function autoSaveNote(noteId: string, newBody: string): Promise<AutoSaveOutcome> {
	return invoke<AutoSaveOutcome>('auto_save_note', { noteId, newBody });
}
