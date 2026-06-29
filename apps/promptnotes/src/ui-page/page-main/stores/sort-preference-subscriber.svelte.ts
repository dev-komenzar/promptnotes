import { listen as tauriListen } from '@tauri-apps/api/event';
import type { SortOrder } from '$lib/note-feed/slices/change-sort-order';
import { feedStore } from './feed.svelte';

export type SortPreferenceChangedPayload = {
	new_sort: SortOrder;
};

export type SortPreferenceChangedListener = (
	handler: (payload: SortPreferenceChangedPayload) => void
) => Promise<() => void>;

export type SortPreferenceSubscriberDeps = {
	listenFn?: SortPreferenceChangedListener;
	onSortChanged?: (sort: SortOrder) => void;
	getCurrentSort?: () => SortOrder;
};

const SORT_PREFERENCE_CHANGED_EVENT = 'settings:sort_preference_changed';

const defaultListenFn: SortPreferenceChangedListener = async (handler) =>
	tauriListen<SortPreferenceChangedPayload>(SORT_PREFERENCE_CHANGED_EVENT, (event) => {
		handler(event.payload);
	});

function sortEquals(a: SortOrder, b: SortOrder): boolean {
	return a.field === b.field && a.direction === b.direction;
}

export function createSortPreferenceSubscriber(deps: SortPreferenceSubscriberDeps = {}) {
	const listenFn = deps.listenFn ?? defaultListenFn;
	const onSortChanged = deps.onSortChanged ?? ((sort) => feedStore.hydrateSort(sort));
	const getCurrentSort = deps.getCurrentSort ?? (() => feedStore.sort);

	let unsubscribe: (() => void) | null = null;

	async function start(): Promise<void> {
		if (unsubscribe) return;
		try {
			unsubscribe = await listenFn((payload) => {
				const current = getCurrentSort();
				if (sortEquals(current, payload.new_sort)) return;
				onSortChanged(payload.new_sort);
			});
		} catch {
			// listen 失敗は silent (非 Tauri 環境 / test 環境)
		}
	}

	function stop(): void {
		unsubscribe?.();
		unsubscribe = null;
	}

	return { start, stop };
}
