import { describe, expect, it, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { copyNoteBody } from '$lib/note-capture/slices/copy-note-body';
import DraftRegion from '../regions/DraftRegion.svelte';
import { toastStore } from '../stores/toasts.svelte';
import { submitShortcutHint } from '../submit-shortcut-hint';

vi.mock('$lib/note-capture/slices/copy-note-body', () => ({
	copyNoteBody: vi.fn().mockResolvedValue(undefined)
}));

function makeFakeFeed() {
	return { prependNote: vi.fn() };
}

function activeInDraftBody(): boolean {
	const el = document.activeElement;
	return !!el?.closest('[data-testid="page.page-main.draft-body"]');
}

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

describe('component:DraftRegion focus after submit', () => {
	// spec: screen-1.md#cross-draft-submit — submit 後も Draft 本文にフォーカスが残る (連続入力)
	it('spec#cross-draft-submit — + Add 成功後も Draft 本文エディタにフォーカスが戻る', async () => {
		const store = makeFakeStore();
		store.submit.mockResolvedValue({
			outcome: 'created',
			id: '20260620120000',
			created_at: '2026-06-20T12:00:00Z'
		});
		const feed = makeFakeFeed();
		render(DraftRegion, { store: store as never, feed: feed as never });

		await page.getByRole('button', { name: /Add new note/ }).click();

		await vi.waitFor(() => {
			expect(feed.prependNote).toHaveBeenCalledTimes(1);
		});
		await vi.waitFor(() => {
			expect(activeInDraftBody()).toBe(true);
		});
	});

	it('spec#cross-draft-submit — + Add 失敗時・no-op でも Draft 本文エディタにフォーカスが戻る', async () => {
		const store = makeFakeStore();
		store.submit.mockResolvedValue({ outcome: 'no_op' });
		render(DraftRegion, { store: store as never, feed: makeFakeFeed() as never });

		await page.getByRole('button', { name: /Add new note/ }).click();

		await vi.waitFor(() => {
			expect(store.submit).toHaveBeenCalledTimes(1);
		});
		await vi.waitFor(() => {
			expect(activeInDraftBody()).toBe(true);
		});
	});

	it('spec#cross-draft-submit — + Add の submit が reject しても Draft 本文エディタにフォーカスが戻る', async () => {
		const store = makeFakeStore();
		store.submit.mockRejectedValue(new Error('boom'));
		const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
		render(DraftRegion, { store: store as never, feed: makeFakeFeed() as never });

		await page.getByRole('button', { name: /Add new note/ }).click();

		await vi.waitFor(() => {
			expect(errorSpy).toHaveBeenCalled();
		});
		await vi.waitFor(() => {
			expect(activeInDraftBody()).toBe(true);
		});
		errorSpy.mockRestore();
	});

	// 回帰防止: キーボード経路は元々 CodeMirror にフォーカスが残る
	it('spec#cross-draft-submit — Cmd+Enter 成功後も Draft 本文エディタにフォーカスが残る', async () => {
		const store = makeFakeStore();
		store.submit.mockResolvedValue({
			outcome: 'created',
			id: '20260620120000',
			created_at: '2026-06-20T12:00:00Z'
		});
		const feed = makeFakeFeed();
		const { container } = render(DraftRegion, { store: store as never, feed: feed as never });

		const content = container.querySelector<HTMLElement>('.cm-content');
		expect(content).toBeTruthy();
		content!.focus();
		await userEvent.keyboard('{Control>}{Enter}{/Control}');

		await vi.waitFor(() => {
			expect(feed.prependNote).toHaveBeenCalledTimes(1);
		});
		expect(activeInDraftBody()).toBe(true);
	});
});

describe('component:DraftRegion clipboard copy after submit', () => {
	function createdStore() {
		const store = makeFakeStore();
		store.submit.mockResolvedValue({
			outcome: 'created',
			id: '20260620120000',
			created_at: '2026-06-20T12:00:00Z'
		});
		return store;
	}

	it('spec#tp-copy-on-create — 作成成功後に copy-note-body を呼び Copied toast を出す', async () => {
		toastStore.reset();
		const feed = makeFakeFeed();
		render(DraftRegion, { store: createdStore() as never, feed: feed as never });

		await page.getByRole('button', { name: /Add new note/ }).click();

		await vi.waitFor(() => {
			expect(toastStore.entries.map((e) => e.kind)).toEqual(['copied']);
		});
		expect(copyNoteBody).toHaveBeenCalledWith('20260620120000');
		expect(feed.prependNote).toHaveBeenCalledTimes(1);
		toastStore.reset();
	});

	it('spec#tp-copy-failed-toast — clipboard 書き込み失敗で Copy failed toast を出し、ノート作成は成功している', async () => {
		toastStore.reset();
		vi.mocked(copyNoteBody).mockRejectedValueOnce(new Error('clipboard'));
		const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
		const feed = makeFakeFeed();
		render(DraftRegion, { store: createdStore() as never, feed: feed as never });

		await page.getByRole('button', { name: /Add new note/ }).click();

		await vi.waitFor(() => {
			expect(toastStore.entries.map((e) => e.kind)).toEqual(['copy-failed']);
		});
		expect(feed.prependNote).toHaveBeenCalledTimes(1);
		errorSpy.mockRestore();
		toastStore.reset();
	});
});

describe('component:DraftRegion submit via Cmd+Enter', () => {
	async function submitWithKeyboard(container: HTMLElement) {
		const content = container.querySelector<HTMLElement>('.cm-content');
		expect(content).toBeTruthy();
		content!.focus();
		await userEvent.keyboard('{Control>}{Enter}{/Control}');
	}

	it('spec#tp-copy-failed-toast — Cmd+Enter 経路でも clipboard 書き込み失敗で Copy failed toast を出し、ノート作成は成功している', async () => {
		toastStore.reset();
		vi.mocked(copyNoteBody).mockRejectedValueOnce(new Error('clipboard'));
		const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
		const store = makeFakeStore();
		store.submit.mockResolvedValue({
			outcome: 'created',
			id: '20260620120000',
			created_at: '2026-06-20T12:00:00Z'
		});
		const feed = makeFakeFeed();
		const { container } = render(DraftRegion, { store: store as never, feed: feed as never });

		await submitWithKeyboard(container);

		await vi.waitFor(() => {
			expect(toastStore.entries.map((e) => e.kind)).toEqual(['copy-failed']);
		});
		expect(feed.prependNote).toHaveBeenCalledTimes(1);
		errorSpy.mockRestore();
		toastStore.reset();
	});

	it('spec#tp-empty-body-noop — 空 Draft の Cmd+Enter は no_op で Feed に変化なし / コピーも Toast もしない', async () => {
		toastStore.reset();
		vi.mocked(copyNoteBody).mockClear();
		const store = makeFakeStore();
		store.submit.mockResolvedValue({ outcome: 'no_op' });
		const feed = makeFakeFeed();
		const { container } = render(DraftRegion, { store: store as never, feed: feed as never });

		await submitWithKeyboard(container);

		await vi.waitFor(() => {
			expect(store.submit).toHaveBeenCalledTimes(1);
		});
		// submit の解決後に後続処理が走る余地を与えてから、何も起きていないことを確認する
		await new Promise((r) => setTimeout(r, 50));
		expect(feed.prependNote).not.toHaveBeenCalled();
		expect(copyNoteBody).not.toHaveBeenCalled();
		expect(toastStore.entries).toEqual([]);
	});
});
