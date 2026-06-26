import {
	changeSortOrder,
	type SortDirection,
	type SortField,
	type SortOrder
} from '$lib/note-feed/slices/change-sort-order';
import {
	updateFeedFilter,
	type DateRangeFilter,
	type NoteFeedFilterDto,
	type UpdateFeedFilterError
} from '$lib/note-feed/slices/update-feed-filter';

export type FeedStoreDeps = {
	updateFilter?: typeof updateFeedFilter;
	changeSort?: typeof changeSortOrder;
};

const DEFAULT_FILTER: NoteFeedFilterDto = {
	query: null,
	date_range: { kind: 'all' },
	tag: null
};

const DEFAULT_SORT: SortOrder = { field: 'created_at', direction: 'desc' };

export type NoteSummary = {
	id: string;
	body: string;
	tags: string[];
	created_at: string;
	updated_at: string;
};

export type FeedStore = ReturnType<typeof createFeedStore>;

const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * In-memory NoteFeed.visible_notes resolution (page-main-feed sub-task の暫定実装)。
 * 本格的な永続層 hydration + Rust 側 visible_notes 解決は list-feed slice (ori-64x.10) で対応。
 *
 * - query: case-insensitive substring (body + tags)
 * - date_range: created_at が範囲内
 * - tag: 完全一致 (1 タグ)
 * - sort: field / direction
 */
export function applyFeedFilter(
	notes: readonly NoteSummary[],
	filter: NoteFeedFilterDto,
	sort: SortOrder,
	now: number = Date.now()
): NoteSummary[] {
	let result = [...notes];

	if (filter.query !== null && filter.query.trim() !== '') {
		const needle = filter.query.toLocaleLowerCase('en');
		result = result.filter((note) => {
			if (note.body.toLocaleLowerCase('en').includes(needle)) return true;
			return note.tags.some((tag) => tag.toLocaleLowerCase('en').includes(needle));
		});
	}

	if (filter.tag !== null) {
		const target = filter.tag;
		result = result.filter((note) => note.tags.includes(target));
	}

	switch (filter.date_range.kind) {
		case 'last_7_days':
			result = filterByDays(result, now, 7);
			break;
		case 'last_30_days':
			result = filterByDays(result, now, 30);
			break;
		case 'last_90_days':
			result = filterByDays(result, now, 90);
			break;
		case 'custom': {
			const from = Date.parse(filter.date_range.from);
			const to = Date.parse(filter.date_range.to);
			if (!Number.isNaN(from) && !Number.isNaN(to)) {
				result = result.filter((note) => {
					const ts = Date.parse(note.created_at);
					return !Number.isNaN(ts) && ts >= from && ts <= to;
				});
			}
			break;
		}
		case 'all':
		default:
			break;
	}

	const field = sort.field === 'updated_at' ? 'updated_at' : 'created_at';
	const sign = sort.direction === 'asc' ? 1 : -1;
	result.sort((a, b) => {
		const av = Date.parse(a[field]);
		const bv = Date.parse(b[field]);
		return sign * (av - bv);
	});

	return result;
}

function filterByDays(notes: NoteSummary[], now: number, days: number): NoteSummary[] {
	const since = now - days * DAY_MS;
	return notes.filter((note) => {
		const ts = Date.parse(note.created_at);
		return !Number.isNaN(ts) && ts >= since;
	});
}

export function createFeedStore(deps: FeedStoreDeps = {}) {
	const updateFilter = deps.updateFilter ?? updateFeedFilter;
	const changeSort = deps.changeSort ?? changeSortOrder;

	let filter = $state<NoteFeedFilterDto>({ ...DEFAULT_FILTER });
	let sort = $state<SortOrder>({ ...DEFAULT_SORT });
	let lastError = $state<UpdateFeedFilterError | null>(null);
	let notes = $state<NoteSummary[]>([]);
	let focusedNoteId = $state<string | null>(null);

	async function setQuery(raw: string): Promise<void> {
		try {
			filter = await updateFilter({ kind: 'set_query', raw });
			lastError = null;
		} catch (err) {
			lastError = err as UpdateFeedFilterError;
		}
	}

	async function setDateRange(range: DateRangeFilter): Promise<void> {
		filter = await updateFilter({ kind: 'set_date_range', range });
		lastError = null;
	}

	async function setTag(raw: string | null): Promise<void> {
		try {
			filter = await updateFilter({ kind: 'set_tag', raw });
			lastError = null;
		} catch (err) {
			lastError = err as UpdateFeedFilterError;
		}
	}

	async function clearAll(): Promise<void> {
		filter = await updateFilter({ kind: 'clear_all' });
		lastError = null;
	}

	async function setSortField(field: SortField): Promise<void> {
		const next = { ...sort, field };
		const result = await changeSort({ new_sort: next });
		sort = result.sort;
	}

	async function setSortDirection(direction: SortDirection): Promise<void> {
		const next = { ...sort, direction };
		const result = await changeSort({ new_sort: next });
		sort = result.sort;
	}

	function hydrateSort(next: SortOrder): void {
		sort = { ...next };
	}

	function prependNote(note: NoteSummary): void {
		notes = [note, ...notes];
		focusedNoteId = note.id;
	}

	function setFocus(id: string | null): void {
		focusedNoteId = id;
	}

	function patchNote(id: string, patch: Partial<NoteSummary>): void {
		notes = notes.map((n) => (n.id === id ? { ...n, ...patch } : n));
	}

	function applyAssignTag(id: string, tags: string[], updated_at: string): void {
		patchNote(id, { tags: [...tags], updated_at });
	}

	function applyRemoveTag(id: string, tags: string[], updated_at: string): void {
		patchNote(id, { tags: [...tags], updated_at });
	}

	function applyAutoSave(id: string, updated_at: string): void {
		patchNote(id, { updated_at });
	}

	function applyBodyEdit(id: string, body: string): void {
		patchNote(id, { body });
	}

	function applyDelete(id: string): void {
		notes = notes.filter((n) => n.id !== id);
		if (focusedNoteId === id) focusedNoteId = null;
	}

	return {
		get filter() {
			return filter;
		},
		get sort() {
			return sort;
		},
		get lastError() {
			return lastError;
		},
		get notes() {
			return notes;
		},
		get visibleNotes() {
			return applyFeedFilter(notes, filter, sort);
		},
		get focusedNoteId() {
			return focusedNoteId;
		},
		setQuery,
		setDateRange,
		setTag,
		clearAll,
		setSortField,
		setSortDirection,
		hydrateSort,
		prependNote,
		setFocus,
		applyAssignTag,
		applyRemoveTag,
		applyAutoSave,
		applyBodyEdit,
		applyDelete
	};
}

export const feedStore = createFeedStore();
