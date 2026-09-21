import { invoke } from '@tauri-apps/api/core';

export type RecreateNoteOutcome = {
	outcome: 'recreated';
	id: string;
	body: string;
	tags: string[];
	created_at: string;
	updated_at: string;
};

export type RecreateNoteError =
	| { kind: 'invalid_note_id'; raw: string }
	| { kind: 'invalid_body'; reason: string }
	| { kind: 'invalid_tag'; raw: string; reason: string }
	| { kind: 'persist_error'; path: string; reason: string };

/**
 * Invoke the `recreate_note` Tauri command (scenario S20).
 *
 * 外部削除された Note の `.md` を **元の NoteId のまま** 現在の編集中 body で再作成する。
 * `note_id` は `YYYYMMDDhhmmss` 形式で、Rust 側が `created_at` として復元するため
 * `Note::from_persisted` の id 導出 (I-N2) が元の id と一致する。
 */
export async function recreateNote(
	noteId: string,
	rawBody: string,
	rawTags: string[]
): Promise<RecreateNoteOutcome> {
	return invoke<RecreateNoteOutcome>('recreate_note', { noteId, rawBody, rawTags });
}
