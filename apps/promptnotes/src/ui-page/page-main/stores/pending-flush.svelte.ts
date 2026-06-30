/**
 * Pending flush registry — S13 quit orchestration の前段。
 *
 * domain/validation.md#s13-quit-flush は「全 EDITING ブロックを Flush →
 * Note::edit_body を A, B, C の順に同期実行」を要求する。
 * flush-note slice の use case (C-FL11) は 1 件しか処理しないため、
 * 複数 Note 順次処理は composition root 側の責務。
 *
 * 各 Block.svelte は pendingBody (debounce 待ち) を抱えている間だけ
 * `register` し、空になったら `unregister` する。quit hook
 * (PageMain.svelte の onCloseRequested) は `flushAll('app_quit')` を
 * await して順次 flush する。
 *
 * spec.md#impl-quit-orchestration:
 *   Tauri 側: app.on_window_event(WindowEvent::CloseRequested) で
 *   全 EDITING Note の FlushNoteCommand を順次呼び、最後の結果を待って
 *   から app.exit(0) する
 *
 * 採用案 (案 1, ori-73q): Rust 側は invoke handler のみ。
 * onCloseRequested を JS で intercept し、preventDefault → flushAll →
 * window.destroy() の流れを frontend が orchestrate する。
 */

import { SvelteMap } from 'svelte/reactivity';
import type { FlushTrigger } from '$lib/note-capture/slices/flush-note';

export type PendingFlushFn = (trigger: FlushTrigger) => Promise<void>;

export type PendingFlushRegistry = ReturnType<typeof createPendingFlushRegistry>;

export function createPendingFlushRegistry() {
	const entries = new SvelteMap<string, PendingFlushFn>();

	function register(noteId: string, flush: PendingFlushFn): void {
		entries.set(noteId, flush);
	}

	function unregister(noteId: string): void {
		entries.delete(noteId);
	}

	/**
	 * 全 entry を **挿入順** に順次 await する。並列化しないのは S13 Then の
	 * 「A, B, C の順に同期実行 → event が A, B, C 順に連続発行」要件に従うため
	 * (domain/validation.md#s13-then)。1 件失敗しても残りを止めない
	 * (catch して swallow): 1 件の I/O 失敗で他 Note のデータを失わせないため。
	 */
	async function flushAll(trigger: FlushTrigger): Promise<void> {
		const snapshot = Array.from(entries.values());
		for (const flush of snapshot) {
			try {
				await flush(trigger);
			} catch {
				// 個別失敗は swallow して次へ。MVP は silent (toast 化は別 issue)
			}
		}
	}

	function size(): number {
		return entries.size;
	}

	function clear(): void {
		entries.clear();
	}

	return { register, unregister, flushAll, size, clear };
}

export const pendingFlushRegistry = createPendingFlushRegistry();
