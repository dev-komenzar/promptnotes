import { describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import PageMain from '../PageMain.svelte';

const noopLoadSettings = async () => ({
	storage_dir: '/tmp',
	theme: 'System' as const,
	sort_preference: { field: 'created_at' as const, direction: 'desc' as const }
});

describe('page:page-main shell', () => {
	it('spec#tp-mount — 4 region (toolbar / draft / feed / toast) を DOM に mount する', async () => {
		render(PageMain, { loadSettingsFn: noopLoadSettings });

		for (const id of ['region-toolbar', 'region-draft', 'region-feed', 'region-toast']) {
			await expect.element(page.getByTestId(id)).toBeInTheDocument();
		}
	});

	it('spec#tp-no-multi-pane — page-main / region-feed はそれぞれ単一 instance のみ存在する', async () => {
		const { container } = render(PageMain, { loadSettingsFn: noopLoadSettings });

		const pageMain = container.querySelectorAll('[data-testid="page-main"]');
		const feed = container.querySelectorAll('[data-testid="region-feed"]');
		const draft = container.querySelectorAll('[data-testid="region-draft"]');

		expect(pageMain.length).toBe(1);
		expect(feed.length).toBe(1);
		expect(draft.length).toBe(1);
	});

	it('spec#fields-toolbar — toolbar に検索 / 期間 / sort / 設定 / ClearAll 入力が揃う', async () => {
		render(PageMain, { loadSettingsFn: noopLoadSettings });

		for (const id of [
			'screen-1-toolbar-search-query',
			'screen-1-toolbar-date-range',
			'screen-1-toolbar-sort-field',
			'screen-1-toolbar-sort-direction',
			'screen-1-toolbar-settings-button',
			'screen-1-toolbar-clear-all'
		]) {
			await expect.element(page.getByTestId(id)).toBeInTheDocument();
		}
	});
});
