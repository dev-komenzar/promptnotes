import { describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import DraftRegion from '../regions/DraftRegion.svelte';
import { submitShortcutHint } from '../submit-shortcut-hint';

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

		const hint = submitShortcutHint(navigator.userAgent);
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
