import { describe, expect, it, vi } from 'vitest';
import {
	createSortPreferenceSubscriber,
	type SortPreferenceChangedListener,
	type SortPreferenceChangedPayload
} from './sort-preference-subscriber.svelte';
import type { SortOrder } from '$lib/note-feed/slices/change-sort-order';

function makePayload(
	overrides: Partial<SortPreferenceChangedPayload> = {}
): SortPreferenceChangedPayload {
	return {
		new_sort: { field: 'updated_at', direction: 'asc' },
		...overrides
	};
}

type Capture = {
	emit: (payload: SortPreferenceChangedPayload) => void;
	unsubscribe: ReturnType<typeof vi.fn>;
	listenFn: SortPreferenceChangedListener;
};

function makeListenCapture(): Capture {
	let stored: ((payload: SortPreferenceChangedPayload) => void) | null = null;
	const unsubscribe = vi.fn();
	const listenFn: SortPreferenceChangedListener = async (handler) => {
		stored = handler;
		return unsubscribe;
	};
	return {
		emit: (payload) => {
			if (!stored) throw new Error('listenFn was not awaited before emit');
			stored(payload);
		},
		unsubscribe,
		listenFn
	};
}

const DEFAULT_SORT: SortOrder = { field: 'created_at', direction: 'desc' };
const DIFFERENT_SORT: SortOrder = { field: 'updated_at', direction: 'asc' };

describe('page:page-main sort-preference-subscriber', () => {
	it('domain-events#sort-preference-changed-subscribers — 異なる sort を受信したら onSortChanged を呼ぶ', async () => {
		const cap = makeListenCapture();
		const onSortChanged = vi.fn();
		const getCurrentSort = vi.fn().mockReturnValue(DEFAULT_SORT);
		const subscriber = createSortPreferenceSubscriber({
			listenFn: cap.listenFn,
			onSortChanged,
			getCurrentSort
		});

		await subscriber.start();
		cap.emit(makePayload({ new_sort: DIFFERENT_SORT }));

		expect(onSortChanged).toHaveBeenCalledExactlyOnceWith(DIFFERENT_SORT);
	});

	it('domain-events#sort-preference-changed-subscribers — 同一 sort は skip (冪等 reapply)', async () => {
		const cap = makeListenCapture();
		const onSortChanged = vi.fn();
		const getCurrentSort = vi.fn().mockReturnValue(DIFFERENT_SORT);
		const subscriber = createSortPreferenceSubscriber({
			listenFn: cap.listenFn,
			onSortChanged,
			getCurrentSort
		});

		await subscriber.start();
		cap.emit(makePayload({ new_sort: DIFFERENT_SORT }));

		expect(onSortChanged).not.toHaveBeenCalled();
	});

	it('domain-events#sort-preference-changed-subscribers — NoteFeed.change_sort 経路の重複適用を防ぐ', async () => {
		const cap = makeListenCapture();
		const onSortChanged = vi.fn();
		const getCurrentSort = vi.fn().mockReturnValue(DEFAULT_SORT);
		const subscriber = createSortPreferenceSubscriber({
			listenFn: cap.listenFn,
			onSortChanged,
			getCurrentSort
		});

		await subscriber.start();
		cap.emit(makePayload({ new_sort: DIFFERENT_SORT }));
		getCurrentSort.mockReturnValue(DIFFERENT_SORT);
		cap.emit(makePayload({ new_sort: DIFFERENT_SORT }));

		expect(onSortChanged).toHaveBeenCalledTimes(1);
		expect(onSortChanged).toHaveBeenLastCalledWith(DIFFERENT_SORT);
	});

	it('lifecycle — stop で unsubscribe が 1 回呼ばれる', async () => {
		const cap = makeListenCapture();
		const subscriber = createSortPreferenceSubscriber({
			listenFn: cap.listenFn,
			onSortChanged: vi.fn(),
			getCurrentSort: vi.fn().mockReturnValue(DEFAULT_SORT)
		});

		await subscriber.start();
		subscriber.stop();

		expect(cap.unsubscribe).toHaveBeenCalledTimes(1);
	});

	it('lifecycle — start を 2 回呼んでも listenFn は 1 回のみ', async () => {
		const listenFn = vi.fn<SortPreferenceChangedListener>(async () => () => {});
		const subscriber = createSortPreferenceSubscriber({
			listenFn,
			onSortChanged: vi.fn(),
			getCurrentSort: vi.fn().mockReturnValue(DEFAULT_SORT)
		});

		await subscriber.start();
		await subscriber.start();

		expect(listenFn).toHaveBeenCalledTimes(1);
	});

	it('lifecycle — stop 後に start を呼ぶと再購読できる', async () => {
		const cap = makeListenCapture();
		const subscriber = createSortPreferenceSubscriber({
			listenFn: cap.listenFn,
			onSortChanged: vi.fn(),
			getCurrentSort: vi.fn().mockReturnValue(DEFAULT_SORT)
		});

		await subscriber.start();
		subscriber.stop();
		await subscriber.start();

		expect(cap.unsubscribe).toHaveBeenCalledTimes(1);
	});

	it('listen failure — listenFn が reject しても throw せず silent', async () => {
		const failingListen: SortPreferenceChangedListener = async () => {
			throw new Error('boom');
		};
		const subscriber = createSortPreferenceSubscriber({
			listenFn: failingListen,
			onSortChanged: vi.fn(),
			getCurrentSort: vi.fn().mockReturnValue(DEFAULT_SORT)
		});

		await expect(subscriber.start()).resolves.toBeUndefined();
	});
});
