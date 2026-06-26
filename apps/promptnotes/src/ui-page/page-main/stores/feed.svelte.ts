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
		setFocus
	};
}

export const feedStore = createFeedStore();
