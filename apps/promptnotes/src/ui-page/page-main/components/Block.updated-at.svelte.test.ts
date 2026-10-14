import { describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import Block from './Block.svelte';
import type { NoteSummary } from '../stores/feed.svelte';
import { createFeedStore } from '../stores/feed.svelte';
import { createFocusStore } from '../stores/focus.svelte';
import { createPendingFlushRegistry } from '../stores/pending-flush.svelte';

const note: NoteSummary = {
	id: '20260628_120000',
	body: 'hello world',
	tags: [],
	created_at: '2026-06-28 12:00:00',
	updated_at: '2026-06-29 08:30:00'
};

function setup() {
	const feed = createFeedStore();
	feed.hydrateNotes([note]);
	const focus = createFocusStore();
	render(Block, {
		note,
		feed,
		focus,
		onTagFilter: () => undefined,
		pendingFlush: createPendingFlushRegistry()
	});
	return { focus };
}

describe('component:Block spec.md#tp-block-updated-at', () => {
	it('hover していない間 tooltip は存在しない', async () => {
		setup();
		const tooltip = page.getByTestId('page.page-main.block-updated-at');
		await expect.element(tooltip).not.toBeInTheDocument();
	});

	it('createdAt を hover すると updatedAt の tooltip が可視になる', async () => {
		setup();
		await page.getByTestId('page.page-main.block-created-at').hover();
		const tooltip = page.getByTestId('page.page-main.block-updated-at');
		await expect.element(tooltip).toBeVisible();
		await expect.element(tooltip).toHaveTextContent(note.updated_at);
	});

	it('tooltip をクリックしても EDITING に遷移しない', async () => {
		const { focus } = setup();
		await page.getByTestId('page.page-main.block-created-at').hover();
		await page.getByTestId('page.page-main.block-updated-at').click();
		expect(focus.stateOf(note.id)).not.toBe('EDITING');
	});
});
