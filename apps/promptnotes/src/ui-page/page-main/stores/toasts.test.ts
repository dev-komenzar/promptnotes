import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { RestoreDeletedNoteOutcome } from '$lib/note-capture/slices/restore-deleted-note';
import type { NoteSummary } from './feed.svelte';
import { createToastStore } from './toasts.svelte';

function makeNote(id: string, overrides: Partial<NoteSummary> = {}): NoteSummary {
	return {
		id,
		body: `body of ${id}`,
		tags: [],
		created_at: '2026-06-20T00:00:00Z',
		updated_at: '2026-06-20T00:00:00Z',
		...overrides
	};
}

function restored(
	id: string,
	overrides: Partial<RestoreDeletedNoteOutcome> = {}
): RestoreDeletedNoteOutcome {
	return {
		outcome: 'restored',
		id,
		body: `body of ${id}`,
		tags: [],
		updated_at: '2026-06-26T12:00:00Z',
		...overrides
	};
}

describe('page:page-main toast store', () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});
	afterEach(() => {
		vi.useRealTimers();
	});

	it('spec#fields-toast — push で entry が先頭（最新）に積まれる', () => {
		const restore = vi.fn().mockResolvedValue(restored('a'));
		const store = createToastStore({ restoreFn: restore, timeoutMs: 5000 });

		store.push(makeNote('a', { body: 'first' }));
		store.push(makeNote('b', { body: 'second' }));

		expect(store.entries.map((e) => e.id)).toStrictEqual(['b', 'a']);
		expect(store.entries[0].preview).toBe('second');
	});

	it('spec#tp-toast-stack (I-PM7) — 連続削除で複数 entry が独立に積まれる', () => {
		const store = createToastStore({ restoreFn: vi.fn(), timeoutMs: 5000 });
		store.push(makeNote('a'));
		store.push(makeNote('b'));
		store.push(makeNote('c'));
		expect(store.entries).toHaveLength(3);
		expect(store.entries.map((e) => e.id)).toStrictEqual(['c', 'b', 'a']);
	});

	it('spec#cross-toast-display — 1 Toast の Undo は他 Toast の生存に影響しない', async () => {
		const restore = vi.fn().mockResolvedValue(restored('a'));
		const store = createToastStore({ restoreFn: restore, timeoutMs: 5000 });
		store.push(makeNote('a'));
		store.push(makeNote('b'));

		const ok = await store.undo('a');

		expect(ok).toBe(true);
		expect(restore).toHaveBeenCalledWith('a');
		expect(store.entries.map((e) => e.id)).toStrictEqual(['b']);
	});

	it('spec#cross-toast-display — timeout 経過で entry は自動 dismiss される', () => {
		const store = createToastStore({ restoreFn: vi.fn(), timeoutMs: 5000 });
		store.push(makeNote('a'));
		expect(store.entries).toHaveLength(1);

		vi.advanceTimersByTime(4999);
		expect(store.entries).toHaveLength(1);

		vi.advanceTimersByTime(1);
		expect(store.entries).toHaveLength(0);
	});

	it('spec#fields-toast-undo (S7) — dismiss 後の undo は no-op で restore を呼ばない', async () => {
		const restore = vi.fn().mockResolvedValue(restored('a'));
		const store = createToastStore({ restoreFn: restore, timeoutMs: 5000 });
		store.push(makeNote('a'));

		vi.advanceTimersByTime(5000);
		expect(store.entries).toHaveLength(0);

		const ok = await store.undo('a');
		expect(ok).toBe(false);
		expect(restore).not.toHaveBeenCalled();
	});

	it('spec#fields-toast-close — close で個別 dismiss、timer も解除される', () => {
		const store = createToastStore({ restoreFn: vi.fn(), timeoutMs: 5000 });
		store.push(makeNote('a'));
		store.push(makeNote('b'));

		store.dismiss('a');

		expect(store.entries.map((e) => e.id)).toStrictEqual(['b']);
		// timer for 'a' was cleared; advancing time should not throw / re-dismiss
		vi.advanceTimersByTime(10_000);
		expect(store.entries).toHaveLength(0);
	});

	it('Cmd+Z global — undoLatest は entries[0] を undo する', async () => {
		const restore = vi.fn().mockResolvedValue(restored('b'));
		const store = createToastStore({ restoreFn: restore, timeoutMs: 5000 });
		store.push(makeNote('a'));
		store.push(makeNote('b'));

		const ok = await store.undoLatest();

		expect(ok).toBe(true);
		expect(restore).toHaveBeenCalledWith('b');
		expect(store.entries.map((e) => e.id)).toStrictEqual(['a']);
	});

	it('Cmd+Z global (S7) — entries 空のとき undoLatest は no-op', async () => {
		const restore = vi.fn();
		const store = createToastStore({ restoreFn: restore, timeoutMs: 5000 });

		const ok = await store.undoLatest();

		expect(ok).toBe(false);
		expect(restore).not.toHaveBeenCalled();
	});

	it('onRestored — undo 成功で original created_at + restored body/tags/updated_at が渡る', async () => {
		const restore = vi
			.fn()
			.mockResolvedValue(
				restored('a', { body: 'restored body', tags: ['x'], updated_at: '2026-07-01T00:00:00Z' })
			);
		const store = createToastStore({ restoreFn: restore, timeoutMs: 5000 });
		const onRestored = vi.fn();
		store.setOnRestored(onRestored);

		store.push(makeNote('a', { body: 'old', created_at: '2026-06-01T00:00:00Z' }));
		await store.undo('a');

		expect(onRestored).toHaveBeenCalledWith({
			id: 'a',
			body: 'restored body',
			tags: ['x'],
			created_at: '2026-06-01T00:00:00Z',
			updated_at: '2026-07-01T00:00:00Z'
		});
	});

	it('restore 失敗時は silent で entry のみ消える', async () => {
		const restore = vi.fn().mockRejectedValue({ kind: 'no_undo_available', id: 'a' });
		const store = createToastStore({ restoreFn: restore, timeoutMs: 5000 });
		store.push(makeNote('a'));

		const ok = await store.undo('a');

		expect(ok).toBe(false);
		expect(store.entries).toHaveLength(0);
	});

	it('preview は最初の非空行を最大 40 文字までに丸める', () => {
		const store = createToastStore({ restoreFn: vi.fn(), timeoutMs: 5000 });
		const longBody = 'a'.repeat(50);
		store.push(makeNote('a', { body: `\n\n${longBody}\nnext line` }));
		expect(store.entries[0].preview).toBe('a'.repeat(40) + '…');
	});

	it('空 body は (空) として preview される', () => {
		const store = createToastStore({ restoreFn: vi.fn(), timeoutMs: 5000 });
		store.push(makeNote('a', { body: '   \n  \n' }));
		expect(store.entries[0].preview).toBe('(空)');
	});

	it('reset は全 entry と全 timer を破棄する', () => {
		const store = createToastStore({ restoreFn: vi.fn(), timeoutMs: 5000 });
		store.push(makeNote('a'));
		store.push(makeNote('b'));

		store.reset();

		expect(store.entries).toHaveLength(0);
		vi.advanceTimersByTime(10_000);
		expect(store.entries).toHaveLength(0);
	});
});
