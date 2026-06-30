import { invoke } from '@tauri-apps/api/core';

export type AssignTagOutcome =
	| { outcome: 'assigned'; id: string; tags: string[]; updated_at: string }
	| { outcome: 'no_op' };

export type AssignTagError =
	| { kind: 'note_not_found'; id: string }
	| { kind: 'invalid_tag'; name: string; reason: string }
	| { kind: 'load_error'; path: string; reason: string }
	| { kind: 'persist_error'; path: string; reason: string };

/**
 * Invoke the `assign_tag` Tauri command.
 *
 * `raw_tag` を Tag::new で I-N6 (禁止文字 / 空文字) 検証してから Note に
 * 追加する。既に同じ tag が付与済みなら NoOp (C-AT3)、検証失敗は
 * `InvalidTag` を surface。成功時は更新後の tag 列を返却。
 */
export async function assignTag(noteId: string, rawTag: string): Promise<AssignTagOutcome> {
	return invoke<AssignTagOutcome>('assign_tag', { noteId, rawTag });
}
