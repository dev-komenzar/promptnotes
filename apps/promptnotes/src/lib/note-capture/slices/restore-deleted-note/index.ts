import { invoke } from '@tauri-apps/api/core';

export type RestoreDeletedNoteOutcome = {
	outcome: 'restored';
	id: string;
	body: string;
	tags: string[];
	updated_at: string;
};

export type RestoreDeletedNoteError =
	| { kind: 'invalid_note_id'; raw: string }
	| { kind: 'no_undo_available'; id: string }
	| { kind: 'trash_restore_error'; path: string; reason: string }
	| { kind: 'read_error'; path: string; reason: string };

/**
 * Invoke the `restore_deleted_note` Tauri command.
 *
 * Ctrl/Cmd+Z で Undo stack の最後の削除を取り消し、Note を trash から復元する。
 * `note_id` の parse 失敗は `InvalidNoteId` を distinct に surface する
 * (review Pass 1 MED-4: silent UNIX_EPOCH fallback で no-undo に化けないため)。
 */
export async function restoreDeletedNote(noteId: string): Promise<RestoreDeletedNoteOutcome> {
	return invoke<RestoreDeletedNoteOutcome>('restore_deleted_note', { noteId });
}
