import { invoke } from '@tauri-apps/api/core';

export type RemoveTagOutcome =
	| { outcome: 'removed'; id: string; tags: string[]; updated_at: string }
	| { outcome: 'no_op' };

export type RemoveTagError =
	| { kind: 'note_not_found'; id: string }
	| { kind: 'load_error'; path: string; reason: string }
	| { kind: 'persist_error'; path: string; reason: string };

/**
 * Invoke the `remove_tag` Tauri command.
 *
 * Tag chip の × クリックで Note から tag を 1 件外す。`tag_name` が付与
 * されていなければ NoOp (C-RT3)。成功時は更新後の tag 列を返却。
 */
export async function removeTag(
	noteId: string,
	tagName: string
): Promise<RemoveTagOutcome> {
	return invoke<RemoveTagOutcome>('remove_tag', { noteId, tagName });
}
