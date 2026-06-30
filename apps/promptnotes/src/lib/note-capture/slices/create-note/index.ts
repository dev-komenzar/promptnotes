import { invoke } from '@tauri-apps/api/core';

export type CreateNoteOutcome =
	| { outcome: 'created'; id: string; created_at: string }
	| { outcome: 'no_op' };

export type CreateNoteError =
	| { kind: 'invalid_tag'; raw: string; reason: string }
	| { kind: 'invalid_body'; reason: string }
	| { kind: 'persist_error'; path: string; reason: string };

/**
 * Invoke the `create_note` Tauri command.
 *
 * Note Capture BC の Note 新規作成。`raw_body` を NoteBody::new で I-N4 (NFC /
 * 非空) 検証し、`raw_tags` の各要素を Tag::new で I-N6 (禁止文字 / 空文字)
 * 検証する。空 body は NoOp (C-CN3)、検証失敗は `CreateNoteError` を surface。
 * 永続化は app_data_dir/notes/ 配下 (将来 Settings::storage_dir に置換)。
 */
export async function createNote(rawBody: string, rawTags: string[]): Promise<CreateNoteOutcome> {
	return invoke<CreateNoteOutcome>('create_note', { rawBody, rawTags });
}
