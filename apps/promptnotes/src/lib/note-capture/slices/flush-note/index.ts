import { invoke } from '@tauri-apps/api/core';

export type FlushTrigger = 'block_blur' | 'window_blur' | 'app_quit';

export type FlushOutcome =
	| { outcome: 'flushed'; id: string; updated_at: string }
	| { outcome: 'no_op' };

export type FlushError =
	| { kind: 'note_not_found'; id: string }
	| { kind: 'invalid_body'; reason: string }
	| { kind: 'load_error'; path: string; reason: string }
	| { kind: 'persist_error'; path: string; reason: string };

/**
 * Invoke the `flush_note` Tauri command.
 *
 * block focus loss / window blur / app quit の trailing-edge 永続化。
 * debounce を bypass して即時 flush し、`pending_body` が現行と同値なら
 * NoOp (C-FL3)。UI 側 debounce timer の cancel handle は frontend が保持し、
 * Rust 側は記録のみ。
 */
export async function flushNote(
	noteId: string,
	pendingBody: string,
	trigger: FlushTrigger
): Promise<FlushOutcome> {
	return invoke<FlushOutcome>('flush_note', { noteId, pendingBody, trigger });
}
