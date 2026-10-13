import { afterEach, describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import PageMain from '../PageMain.svelte';

const noopLoadSettings = async () => ({
	storage_dir: '/tmp',
	theme: 'System' as const,
	sort_preference: { field: 'created_at' as const, direction: 'desc' as const }
});

const noopListNotes = async () => ({ notes: [] });

afterEach(() => {
	document.documentElement.classList.remove('dark');
});

describe('page:page-main shell', () => {
	it('spec#tp-mount — 4 region (toolbar / draft / feed / toast) を DOM に mount する', async () => {
		render(PageMain, { loadSettingsFn: noopLoadSettings, listNotesFn: noopListNotes });

		for (const id of [
			'page.page-main.region-toolbar',
			'page.page-main.region-draft',
			'page.page-main.region-feed',
			'page.page-main.region-toast'
		]) {
			await expect.element(page.getByTestId(id)).toBeInTheDocument();
		}
	});

	it('spec#tp-no-multi-pane — page-main / region-feed はそれぞれ単一 instance のみ存在する', async () => {
		const { container } = render(PageMain, {
			loadSettingsFn: noopLoadSettings,
			listNotesFn: noopListNotes
		});

		const pageMain = container.querySelectorAll('[data-testid="page.page-main"]');
		const feed = container.querySelectorAll('[data-testid="page.page-main.region-feed"]');
		const draft = container.querySelectorAll('[data-testid="page.page-main.region-draft"]');

		expect(pageMain.length).toBe(1);
		expect(feed.length).toBe(1);
		expect(draft.length).toBe(1);
	});

	it('spec#fields-toolbar — toolbar に検索 / 期間 / sort / 設定 / ClearAll 入力が揃う', async () => {
		render(PageMain, { loadSettingsFn: noopLoadSettings, listNotesFn: noopListNotes });

		for (const id of [
			'page.page-main.toolbar-search-query',
			'page.page-main.toolbar-date-range',
			'page.page-main.toolbar-sort-field',
			'page.page-main.toolbar-sort-direction',
			'page.page-main.toolbar-settings-button',
			'page.page-main.toolbar-clear-all'
		]) {
			await expect.element(page.getByTestId(id)).toBeInTheDocument();
		}
	});
});

describe('page:page-main theme wiring', () => {
	it('spec#tp-theme-apply TP-T1 — load-settings で Dark → <html> に dark class が付与される', async () => {
		const loadDark = async () => ({
			storage_dir: '/tmp',
			theme: 'Dark' as const,
			sort_preference: { field: 'created_at' as const, direction: 'desc' as const }
		});

		render(PageMain, { loadSettingsFn: loadDark, listNotesFn: noopListNotes });

		await expect.poll(() => document.documentElement.classList.contains('dark')).toBe(true);
	});

	it('spec#tp-theme-apply TP-T2 — load-settings で Light → <html> から dark class が削除される', async () => {
		document.documentElement.classList.add('dark');
		const loadLight = async () => ({
			storage_dir: '/tmp',
			theme: 'Light' as const,
			sort_preference: { field: 'created_at' as const, direction: 'desc' as const }
		});

		render(PageMain, { loadSettingsFn: loadLight, listNotesFn: noopListNotes });

		await expect.poll(() => document.documentElement.classList.contains('dark')).toBe(false);
	});
});

describe('page:page-main keyboard shortcuts', () => {
	it('spec#cross-screen-shortcuts — Cmd/Ctrl+N で Draft にフォーカスする', async () => {
		render(PageMain, { loadSettingsFn: noopLoadSettings, listNotesFn: noopListNotes });

		await expect.element(page.getByTestId('page.page-main.draft-body')).toBeInTheDocument();

		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', ctrlKey: true, bubbles: true }));

		await expect
			.poll(() => {
				const active = document.activeElement;
				if (!active) return false;
				const draftBody = document.querySelector('[data-testid="page.page-main.draft-body"]');
				if (!draftBody) return false;
				return draftBody.contains(active) && active.classList.contains('cm-content');
			})
			.toBe(true);

		// Cmd/Ctrl+N はノートを作成しないことを確認
		await expect.element(page.getByTestId('page.page-main.block')).not.toBeInTheDocument();
	});
});
