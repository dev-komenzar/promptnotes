import { invoke } from '@tauri-apps/api/core';

export type DeletedNoteDto = {
	id: string;
	original_path: string;
};

export type DeleteNoteError =
	| { kind: 'invalid_note_id'; raw: string }
	| { kind: 'note_not_found'; id: string }
	| {
			kind: 'trash_error';
			path: string;
			variant: 'permission_denied' | 'io' | 'unsupported';
			reason: string;
	  };

/**
 * Invoke the `delete_note` Tauri command. Resolves with `DeletedNoteDto`
 * on success (the Undo handle), rejects with a `DeleteNoteError` payload
 * mirroring spec.md#io-errors.
 *
 * Side effect: the note file is moved to `<storage_dir>/trash/<id>.md`
 * and pushed onto the process-wide Undo stack (I-N7, I-DN8).
 */
export async function deleteNote(noteId: string): Promise<DeletedNoteDto> {
	return invoke<DeletedNoteDto>('delete_note', { noteId });
}
