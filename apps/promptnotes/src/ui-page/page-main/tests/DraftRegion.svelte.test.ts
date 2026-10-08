import { describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import DraftRegion from '../regions/DraftRegion.svelte';
import { submitShortcutHint } from '../submit-shortcut-hint';
import { createFocusStore } from '../stores/focus.svelte';

vi.mock('$lib/note-capture/slices/copy-note-body', () => ({
	copyNoteBody: vi.fn().mockResolvedValue(undefined)
}));

function makeFakeStore() {
	return {
		body: '',
		tags: [] as string[],
		submitting: false,
		setBody: vi.fn(),
		addTag: vi.fn(),
		removeTag: vi.fn(),
		submit: vi.fn().mockResolvedValue({ outcome: 'no_op' })
	};
}

describe('component:DraftRegion add button footer', () => {
	it('spec#fields-draft — ショートカットヒントが aria-hidden で表示される', async () => {
		const store = makeFakeStore();
		const { container } = render(DraftRegion, { store: store as never });

		const hint = submitShortcutHint(navigator.platform);
		const el = Array.from(container.querySelectorAll('[aria-hidden="true"]')).find(
			(n) => n.textContent?.trim() === hint
		);
		expect(el).toBeTruthy();
	});

	it('spec#fields-draft — + Add ボタンのクリックで submit が呼ばれる', async () => {
		const store = makeFakeStore();
		render(DraftRegion, { store: store as never });

		await page.getByRole('button', { name: /Add new note/ }).click();

		await vi.waitFor(() => {
			expect(store.submit).toHaveBeenCalledTimes(1);
		});
	});
});

describe('component:DraftRegion submit → focus (I-PM9)', () => {
	it('spec#invariants-cross-region I-PM9 — submit 成功時に新 Block を FOCUSED にする', async () => {
		const store = makeFakeStore();
		store.submit.mockResolvedValue({
			outcome: 'created',
			id: 'new-note-id',
			created_at: '2026-01-01T00:00:00Z'
		});
		const feed = { prependNote: vi.fn() };
		const focus = createFocusStore();
		render(DraftRegion, { store: store as never, feed: feed as never, focus });

		await page.getByRole('button', { name: /Add new note/ }).click();

		await vi.waitFor(() => {
			expect(feed.prependNote).toHaveBeenCalledTimes(1);
		});
		expect(focus.activeId).toBe('new-note-id');
		expect(focus.activeState).toBe('FOCUSED');
	});

	it('spec#invariants-cross-region I-PM9 — submit が no_op の場合はフォーカスを変えない', async () => {
		const store = makeFakeStore();
		const feed = { prependNote: vi.fn() };
		const focus = createFocusStore();
		render(DraftRegion, { store: store as never, feed: feed as never, focus });

		await page.getByRole('button', { name: /Add new note/ }).click();

		await vi.waitFor(() => {
			expect(store.submit).toHaveBeenCalledTimes(1);
		});
		expect(focus.activeId).toBeNull();
	});
});
