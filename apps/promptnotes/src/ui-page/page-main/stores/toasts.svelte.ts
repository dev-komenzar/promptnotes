import { SvelteMap } from 'svelte/reactivity';
import { restoreDeletedNote } from '$lib/note-capture/slices/restore-deleted-note';
import type { NoteSummary } from './feed.svelte';

/**
 * Toast region store — 削除 Undo Toast の縦パイル管理。
 *
 * - I-PM7 / screen-1.md#cross-toast-display: 各 Toast は独立した DeletedNote を保持
 * - tp-toast-stack: 連続削除で複数 Toast が縦に積まれる（新しい順 = 配列先頭）
 * - S7: timeout / close で消えた Toast の Undo は no-op
 */

export type ToastEntry = {
	id: string;
	preview: string;
	createdAt: number;
	originalNote: NoteSummary;
};

export type ToastStoreDeps = {
	restoreFn?: typeof restoreDeletedNote;
	timeoutMs?: number;
};

const DEFAULT_TIMEOUT_MS = 5000;
const PREVIEW_MAX = 40;

function previewBody(body: string, max = PREVIEW_MAX): string {
	const firstLine = body.split('\n').find((line) => line.trim() !== '') ?? '';
	const trimmed = firstLine.trim();
	if (trimmed === '') return '(空)';
	return trimmed.length > max ? trimmed.slice(0, max) + '…' : trimmed;
}

export type RestoredHandler = (note: NoteSummary) => void;
export type ToastStore = ReturnType<typeof createToastStore>;

export function createToastStore(deps: ToastStoreDeps = {}) {
	const restoreFn = deps.restoreFn ?? restoreDeletedNote;
	const timeoutMs = deps.timeoutMs ?? DEFAULT_TIMEOUT_MS;

	let entries = $state<ToastEntry[]>([]);
	let onRestored: RestoredHandler | undefined;
	const timers = new SvelteMap<string, ReturnType<typeof setTimeout>>();

	function clearTimer(id: string): void {
		const t = timers.get(id);
		if (t !== undefined) {
			clearTimeout(t);
			timers.delete(id);
		}
	}

	function dismiss(id: string): void {
		clearTimer(id);
		entries = entries.filter((e) => e.id !== id);
	}

	function push(note: NoteSummary): void {
		// 同一 id が既に積まれている場合は最新側に置き換え（連続削除 → 復元 → 再削除 の防御）
		clearTimer(note.id);
		const entry: ToastEntry = {
			id: note.id,
			preview: previewBody(note.body),
			createdAt: Date.now(),
			originalNote: note
		};
		entries = [entry, ...entries.filter((e) => e.id !== note.id)];
		timers.set(
			note.id,
			setTimeout(() => dismiss(note.id), timeoutMs)
		);
	}

	async function undo(id: string): Promise<boolean> {
		// S7: 既に dismiss されている Toast の Undo は no-op
		const entry = entries.find((e) => e.id === id);
		if (entry === undefined) return false;
		dismiss(id);
		try {
			const outcome = await restoreFn(id);
			onRestored?.({
				id: outcome.id,
				body: outcome.body,
				tags: [...outcome.tags],
				created_at: entry.originalNote.created_at,
				updated_at: outcome.updated_at
			});
			return true;
		} catch {
			// MVP: silent on restore failure (no second-tier toast).
			return false;
		}
	}

	async function undoLatest(): Promise<boolean> {
		const latest = entries[0];
		if (latest === undefined) return false;
		return undo(latest.id);
	}

	function setOnRestored(handler: RestoredHandler | undefined): void {
		onRestored = handler;
	}

	function reset(): void {
		for (const id of [...timers.keys()]) clearTimer(id);
		entries = [];
	}

	return {
		get entries() {
			return entries;
		},
		push,
		dismiss,
		undo,
		undoLatest,
		setOnRestored,
		reset
	};
}

export const toastStore = createToastStore();
