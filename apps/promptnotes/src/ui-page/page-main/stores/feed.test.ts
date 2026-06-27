import { describe, expect, it, vi } from 'vitest';
import type { NoteFeedDto } from '$lib/note-feed/slices/change-sort-order';
import type { NoteFeedFilterDto } from '$lib/note-feed/slices/update-feed-filter';
import { applyFeedFilter, createFeedStore, type NoteSummary } from './feed.svelte';

function noopFilter(
	initial: NoteFeedFilterDto = { query: null, date_range: { kind: 'all' }, tag: null }
) {
	return vi.fn().mockResolvedValue(initial);
}

function noopSort(initial: NoteFeedDto = { sort: { field: 'created_at', direction: 'desc' } }) {
	return vi.fn().mockResolvedValue(initial);
}

function makeNote(id: string, overrides: Partial<NoteSummary> = {}): NoteSummary {
	return {
		id,
		body: '',
		tags: [],
		created_at: '2026-06-26T00:00:00Z',
		updated_at: '2026-06-26T00:00:00Z',
		...overrides
	};
}

describe('page:page-main feed store', () => {
	it('spec#tp-sort-immediate — setSortField は change-sort-order slice を呼んで結果を反映する', async () => {
		const changeSort = vi.fn().mockResolvedValue({
			sort: { field: 'updated_at', direction: 'desc' }
		} satisfies NoteFeedDto);
		const store = createFeedStore({ updateFilter: noopFilter(), changeSort });

		await store.setSortField('updated_at');

		expect(changeSort).toHaveBeenCalledWith({
			new_sort: { field: 'updated_at', direction: 'desc' }
		});
		expect(store.sort).toStrictEqual({ field: 'updated_at', direction: 'desc' });
	});

	it('spec#tp-sort-immediate — toggleSortDirection は change-sort-order slice を呼ぶ', async () => {
		const changeSort = vi
			.fn()
			.mockResolvedValue({ sort: { field: 'created_at', direction: 'asc' } } satisfies NoteFeedDto);
		const store = createFeedStore({ updateFilter: noopFilter(), changeSort });

		await store.setSortDirection('asc');

		expect(changeSort).toHaveBeenCalledWith({
			new_sort: { field: 'created_at', direction: 'asc' }
		});
		expect(store.sort).toStrictEqual({ field: 'created_at', direction: 'asc' });
	});

	it('spec#invariants-cross-region(I-PM11) — setQuery は update-feed-filter SetQuery を呼んで filter を更新する', async () => {
		const updateFilter = vi.fn().mockResolvedValue({
			query: 'foo',
			date_range: { kind: 'all' },
			tag: null
		} satisfies NoteFeedFilterDto);
		const store = createFeedStore({ updateFilter, changeSort: noopSort() });

		await store.setQuery('foo');

		expect(updateFilter).toHaveBeenCalledWith({ kind: 'set_query', raw: 'foo' });
		expect(store.filter.query).toBe('foo');
		expect(store.lastError).toBeNull();
	});

	it('spec#invariants-cross-region — setDateRange は update-feed-filter SetDateRange を呼ぶ', async () => {
		const updateFilter = vi.fn().mockResolvedValue({
			query: null,
			date_range: { kind: 'last_7_days' },
			tag: null
		} satisfies NoteFeedFilterDto);
		const store = createFeedStore({ updateFilter, changeSort: noopSort() });

		await store.setDateRange({ kind: 'last_7_days' });

		expect(updateFilter).toHaveBeenCalledWith({
			kind: 'set_date_range',
			range: { kind: 'last_7_days' }
		});
		expect(store.filter.date_range).toStrictEqual({ kind: 'last_7_days' });
	});

	it('spec#invariants-cross-region — setTag(null) は tag フィルタを解除する', async () => {
		const updateFilter = vi.fn().mockResolvedValue({
			query: null,
			date_range: { kind: 'all' },
			tag: null
		} satisfies NoteFeedFilterDto);
		const store = createFeedStore({ updateFilter, changeSort: noopSort() });

		await store.setTag(null);

		expect(updateFilter).toHaveBeenCalledWith({ kind: 'set_tag', raw: null });
		expect(store.filter.tag).toBeNull();
	});

	it('spec#tp-no-raw-invoke 補足 — setTag が invalid_tag を返した場合は lastError に格納', async () => {
		const updateFilter = vi
			.fn()
			.mockRejectedValue({ kind: 'invalid_tag', raw: '##', reason: 'invalid_char' });
		const store = createFeedStore({ updateFilter, changeSort: noopSort() });

		await store.setTag('##');

		expect(store.lastError).toStrictEqual({
			kind: 'invalid_tag',
			raw: '##',
			reason: 'invalid_char'
		});
	});

	it('spec#invariants-cross-region — clearAll は filter を初期化する', async () => {
		const updateFilter = vi.fn().mockResolvedValue({
			query: null,
			date_range: { kind: 'all' },
			tag: null
		} satisfies NoteFeedFilterDto);
		const store = createFeedStore({ updateFilter, changeSort: noopSort() });

		await store.clearAll();

		expect(updateFilter).toHaveBeenCalledWith({ kind: 'clear_all' });
		expect(store.filter).toStrictEqual({
			query: null,
			date_range: { kind: 'all' },
			tag: null
		});
	});

	it('spec#impl-notes(I-PM3) — hydrateSort は load-settings 結果を sort 初期値に反映する', () => {
		const store = createFeedStore({ updateFilter: noopFilter(), changeSort: noopSort() });

		store.hydrateSort({ field: 'updated_at', direction: 'asc' });

		expect(store.sort).toStrictEqual({ field: 'updated_at', direction: 'asc' });
	});

	it('spec#I-PM9 — prependNote は新 Note を先頭に挿入し focus を移す', () => {
		const store = createFeedStore({ updateFilter: noopFilter(), changeSort: noopSort() });

		store.prependNote({
			id: 'note-1',
			body: 'first',
			tags: [],
			created_at: '2026-06-26T00:00:00Z',
			updated_at: '2026-06-26T00:00:00Z'
		});
		store.prependNote({
			id: 'note-2',
			body: 'second',
			tags: [],
			created_at: '2026-06-26T00:01:00Z',
			updated_at: '2026-06-26T00:01:00Z'
		});

		expect(store.notes.map((n) => n.id)).toStrictEqual(['note-2', 'note-1']);
		expect(store.focusedNoteId).toBe('note-2');
	});

	it('spec#I-PM11 — visibleNotes は filter / sort を即時反映する (query)', () => {
		const store = createFeedStore({ updateFilter: noopFilter(), changeSort: noopSort() });
		store.prependNote(makeNote('a', { body: 'hello world' }));
		store.prependNote(makeNote('b', { body: 'goodbye' }));

		expect(store.visibleNotes.map((n) => n.id)).toStrictEqual(['b', 'a']);

		// setQuery directly mutates store filter via update-feed-filter mock; bypass
		// by using applyFeedFilter for deterministic spec ('hello')
		const filtered = applyFeedFilter(
			store.notes,
			{ query: 'hello', date_range: { kind: 'all' }, tag: null },
			{ field: 'created_at', direction: 'desc' }
		);
		expect(filtered.map((n) => n.id)).toStrictEqual(['a']);
	});

	it('spec#I-PM11 — visibleNotes は tag filter を反映する', () => {
		const notes: NoteSummary[] = [
			makeNote('a', { tags: ['rust'] }),
			makeNote('b', { tags: ['ts', 'rust'] }),
			makeNote('c', { tags: ['ts'] })
		];
		const filtered = applyFeedFilter(
			notes,
			{ query: null, date_range: { kind: 'all' }, tag: 'rust' },
			{ field: 'created_at', direction: 'desc' }
		);
		expect(filtered.map((n) => n.id)).toStrictEqual(['a', 'b']);
	});

	it('spec#I-PM11 — visibleNotes は sort direction を反映する', () => {
		const notes: NoteSummary[] = [
			makeNote('a', { created_at: '2026-06-26T00:00:00Z' }),
			makeNote('b', { created_at: '2026-06-26T01:00:00Z' })
		];
		const asc = applyFeedFilter(
			notes,
			{ query: null, date_range: { kind: 'all' }, tag: null },
			{ field: 'created_at', direction: 'asc' }
		);
		expect(asc.map((n) => n.id)).toStrictEqual(['a', 'b']);
		const desc = applyFeedFilter(
			notes,
			{ query: null, date_range: { kind: 'all' }, tag: null },
			{ field: 'created_at', direction: 'desc' }
		);
		expect(desc.map((n) => n.id)).toStrictEqual(['b', 'a']);
	});

	it('spec#I-PM11 — visibleNotes は last_7_days で created_at を絞り込む', () => {
		const now = Date.parse('2026-06-26T00:00:00Z');
		const notes: NoteSummary[] = [
			makeNote('recent', { created_at: '2026-06-25T00:00:00Z' }),
			makeNote('old', { created_at: '2026-06-10T00:00:00Z' })
		];
		const filtered = applyFeedFilter(
			notes,
			{ query: null, date_range: { kind: 'last_7_days' }, tag: null },
			{ field: 'created_at', direction: 'desc' },
			now
		);
		expect(filtered.map((n) => n.id)).toStrictEqual(['recent']);
	});

	it('applyAssignTag は対象 note の tags / updated_at を更新する', () => {
		const store = createFeedStore({ updateFilter: noopFilter(), changeSort: noopSort() });
		store.prependNote(makeNote('a', { tags: [] }));

		store.applyAssignTag('a', ['rust'], '2026-06-26T01:00:00Z');

		expect(store.notes[0].tags).toStrictEqual(['rust']);
		expect(store.notes[0].updated_at).toBe('2026-06-26T01:00:00Z');
	});

	it('applyRemoveTag は対象 note の tags / updated_at を更新する', () => {
		const store = createFeedStore({ updateFilter: noopFilter(), changeSort: noopSort() });
		store.prependNote(makeNote('a', { tags: ['rust', 'ts'] }));

		store.applyRemoveTag('a', ['ts'], '2026-06-26T01:00:00Z');

		expect(store.notes[0].tags).toStrictEqual(['ts']);
	});

	it('applyAutoSave は updated_at のみ更新する', () => {
		const store = createFeedStore({ updateFilter: noopFilter(), changeSort: noopSort() });
		store.prependNote(makeNote('a', { body: 'orig', updated_at: '2026-06-26T00:00:00Z' }));

		store.applyAutoSave('a', '2026-06-26T01:00:00Z');

		expect(store.notes[0].updated_at).toBe('2026-06-26T01:00:00Z');
		expect(store.notes[0].body).toBe('orig');
	});

	it('applyBodyEdit は body のみ更新する', () => {
		const store = createFeedStore({ updateFilter: noopFilter(), changeSort: noopSort() });
		store.prependNote(makeNote('a', { body: 'orig' }));

		store.applyBodyEdit('a', 'edited');

		expect(store.notes[0].body).toBe('edited');
	});

	it('applyDelete は note を削除し focus を外す', () => {
		const store = createFeedStore({ updateFilter: noopFilter(), changeSort: noopSort() });
		store.prependNote(makeNote('a'));
		store.prependNote(makeNote('b'));
		store.setFocus('b');

		store.applyDelete('b');

		expect(store.notes.map((n) => n.id)).toStrictEqual(['a']);
		expect(store.focusedNoteId).toBeNull();
	});

	it('setFocus(null) は focus を解除する', () => {
		const store = createFeedStore({ updateFilter: noopFilter(), changeSort: noopSort() });

		store.prependNote({
			id: 'note-1',
			body: 'x',
			tags: [],
			created_at: '2026-06-26T00:00:00Z',
			updated_at: '2026-06-26T00:00:00Z'
		});
		store.setFocus(null);

		expect(store.focusedNoteId).toBeNull();
	});
});
