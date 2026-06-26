import { describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import PageMain from '../PageMain.svelte';

describe('page:page-main shell', () => {
	it('spec#tp-mount — 4 region (toolbar / draft / feed / toast) を DOM に mount する', async () => {
		render(PageMain);

		for (const id of ['region-toolbar', 'region-draft', 'region-feed', 'region-toast']) {
			await expect.element(page.getByTestId(id)).toBeInTheDocument();
		}
	});

	it('spec#tp-no-multi-pane — page-main / region-feed はそれぞれ単一 instance のみ存在する', async () => {
		const { container } = render(PageMain);

		const pageMain = container.querySelectorAll('[data-testid="page-main"]');
		const feed = container.querySelectorAll('[data-testid="region-feed"]');
		const draft = container.querySelectorAll('[data-testid="region-draft"]');

		expect(pageMain.length).toBe(1);
		expect(feed.length).toBe(1);
		expect(draft.length).toBe(1);
	});
});
