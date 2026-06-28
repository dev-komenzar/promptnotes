import { describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import Block from './Block.svelte';
import type { NoteSummary } from '../stores/feed.svelte';
import { createFeedStore } from '../stores/feed.svelte';
import { createFocusStore } from '../stores/focus.svelte';
import { createToastStore } from '../stores/toasts.svelte';
import { createPendingFlushRegistry } from '../stores/pending-flush.svelte';

function makeNote(overrides: Partial<NoteSummary> = {}): NoteSummary {
	return {
		id: '20260628_120000',
		body: 'hello world',
		tags: [],
		created_at: '2026-06-28 12:00:00',
		updated_at: '2026-06-28 12:00:00',
		...overrides
	};
}

describe('component:Block ori-6aa delete → toast push', () => {
	it('spec#fields-toast — clicking 🗑 pushes a toast and removes the note from the feed', async () => {
		const note = makeNote();
		const feed = createFeedStore();
		feed.hydrateNotes([note]);
		const focus = createFocusStore();
		const toasts = createToastStore({ timeoutMs: 60_000, restoreFn: vi.fn() });
		const pendingFlush = createPendingFlushRegistry();
		const deleteFn = vi.fn().mockResolvedValue({
			id: note.id,
			original_path: `/tmp/notes/${note.id}.md`
		});

		render(Block, {
			note,
			feed,
			focus,
			onTagFilter: () => undefined,
			deleteFn,
			toasts,
			pendingFlush
		});

		const deleteButton = page.getByTestId('screen-1-block-delete');
		await deleteButton.click();

		await vi.waitFor(() => {
			expect(deleteFn).toHaveBeenCalledWith(note.id);
			expect(toasts.entries.length).toBe(1);
			expect(toasts.entries[0]?.id).toBe(note.id);
			expect(feed.notes.length).toBe(0);
		});
	});

	it('spec#io-errors — backend rejection leaves the note in feed and skips the toast', async () => {
		const note = makeNote();
		const feed = createFeedStore();
		feed.hydrateNotes([note]);
		const focus = createFocusStore();
		const toasts = createToastStore({ timeoutMs: 60_000, restoreFn: vi.fn() });
		const pendingFlush = createPendingFlushRegistry();
		const deleteFn = vi.fn().mockRejectedValue({
			kind: 'trash_error',
			path: '/tmp/notes/x.md',
			variant: 'io',
			reason: 'disk full'
		});
		const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

		render(Block, {
			note,
			feed,
			focus,
			onTagFilter: () => undefined,
			deleteFn,
			toasts,
			pendingFlush
		});

		const deleteButton = page.getByTestId('screen-1-block-delete');
		await deleteButton.click();

		await vi.waitFor(() => {
			expect(deleteFn).toHaveBeenCalledWith(note.id);
		});
		expect(toasts.entries.length).toBe(0);
		expect(feed.notes.length).toBe(1);
		expect(errorSpy).toHaveBeenCalled();
		errorSpy.mockRestore();
	});
});
