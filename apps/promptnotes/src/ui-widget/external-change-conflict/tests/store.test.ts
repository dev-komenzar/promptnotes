import { describe, expect, it, vi } from 'vitest';
import {
	createConflictDialogStore,
	type ConflictDialogStoreDeps,
	type NoteFileModifiedExternallyPayload
} from '../store.svelte';

// ---- helpers ----

function makePayload(
	overrides: Partial<NoteFileModifiedExternallyPayload> = {}
): NoteFileModifiedExternallyPayload {
	return {
		note_id: '20260630120000',
		disk_body_hash: 'abc123def456',
		note_title: '20260630120000.md',
		note_body: 'hello world!!!',
		file_path: '/notes/20260630120000.md',
		detected_at: '2026-06-30T12:00:00Z',
		...overrides
	};
}

type Capture = {
	emit: (payload: NoteFileModifiedExternallyPayload) => void;
	unsubscribe: ReturnType<typeof vi.fn>;
	subscribeFn: NonNullable<ConflictDialogStoreDeps['subscribeFn']>;
};

function makeSubscribeCapture(): Capture {
	let stored: ((payload: NoteFileModifiedExternallyPayload) => void) | null = null;
	const unsubscribe = vi.fn();
	const subscribeFn = async (handler: (payload: NoteFileModifiedExternallyPayload) => void) => {
		stored = handler;
		return unsubscribe;
	};
	return {
		emit: (payload) => {
			if (!stored) throw new Error('subscribeFn was not awaited before emit');
			stored(payload);
		},
		unsubscribe,
		subscribeFn
	};
}

const LOCAL_HASH = 'abc123_local_xyz';
const MATCHING_HASH = 'abc123_local_xyz'; // same as LOCAL_HASH → isStale false
const DIFFERENT_HASH = 'different_hash_999';

function makeStore(deps: Partial<ConflictDialogStoreDeps> = {}) {
	return createConflictDialogStore({
		isStaleFn: (localHash, diskHash) => localHash !== diskHash,
		currentNoteId: () => '20260630120000',
		currentBodyHash: () => LOCAL_HASH,
		onApplyExternal: vi.fn(),
		...deps
	});
}

// ---- tests ----

describe('widget:widget-external-change-conflict store', () => {
	it('spec.md#tp-wc-no-mount-without-event — event 未受信時は conflictPayload=null, state=hidden', async () => {
		const cap = makeSubscribeCapture();
		const store = makeStore({ subscribeFn: cap.subscribeFn });

		await store.start();

		expect(store.conflictPayload).toBeNull();
		expect(store.state).toBe('hidden');
	});

	it('spec.md#tp-wc-event-mounts-dialog — 競合検出（isStale true）で衝突 payload set + state=compare', async () => {
		const cap = makeSubscribeCapture();
		const store = makeStore({ subscribeFn: cap.subscribeFn });

		await store.start();
		const payload = makePayload({ disk_body_hash: DIFFERENT_HASH });
		cap.emit(payload);

		expect(store.conflictPayload).toStrictEqual(payload);
		expect(store.state).toBe('compare');
	});

	it('spec.md#tp-wc-no-conflict-no-mount — isStale が false なら非表示のまま', async () => {
		const cap = makeSubscribeCapture();
		const store = makeStore({ subscribeFn: cap.subscribeFn });

		await store.start();
		cap.emit(makePayload({ disk_body_hash: MATCHING_HASH }));

		expect(store.conflictPayload).toBeNull();
		expect(store.state).toBe('hidden');
	});

	it('spec.md#tp-wc-apply-external — ApplyExternal で onApplyExternal が 1 回呼ばれ hidden に戻る', async () => {
		const cap = makeSubscribeCapture();
		const onApplyExternal = vi.fn();
		const store = makeStore({
			subscribeFn: cap.subscribeFn,
			onApplyExternal
		});

		await store.start();
		const payload = makePayload({ disk_body_hash: DIFFERENT_HASH });
		cap.emit(payload);

		store.selectResolution('ApplyExternal');
		await store.apply();

		expect(onApplyExternal).toHaveBeenCalledExactlyOnceWith(payload);
		expect(store.conflictPayload).toBeNull();
		expect(store.state).toBe('hidden');
	});

	it('spec.md#tp-wc-keep-editing — KeepEditing で callback 呼ばれず hidden に戻る', async () => {
		const cap = makeSubscribeCapture();
		const onApplyExternal = vi.fn();
		const store = makeStore({
			subscribeFn: cap.subscribeFn,
			onApplyExternal
		});

		await store.start();
		cap.emit(makePayload({ disk_body_hash: DIFFERENT_HASH }));

		store.selectResolution('KeepEditing');
		await store.apply();

		expect(onApplyExternal).not.toHaveBeenCalled();
		expect(store.conflictPayload).toBeNull();
		expect(store.state).toBe('hidden');
	});

	it('spec.md#tp-wc-cancel-equals-keep — cancel は resolution 選択に関わらず hidden に戻る', async () => {
		const cap = makeSubscribeCapture();
		const onApplyExternal = vi.fn();
		const store = makeStore({
			subscribeFn: cap.subscribeFn,
			onApplyExternal
		});

		await store.start();
		cap.emit(makePayload({ disk_body_hash: DIFFERENT_HASH }));
		store.selectResolution('ApplyExternal');

		store.cancel();

		expect(onApplyExternal).not.toHaveBeenCalled();
		expect(store.conflictPayload).toBeNull();
		expect(store.state).toBe('hidden');
	});

	it('spec.md#tp-wc-duplicate-ignored — 同一 note_id の重複 event は無視', async () => {
		const cap = makeSubscribeCapture();
		const store = makeStore({ subscribeFn: cap.subscribeFn });

		await store.start();
		const firstPayload = makePayload({
			note_id: '20260630120000',
			disk_body_hash: DIFFERENT_HASH,
			note_body: 'first change'
		});
		cap.emit(firstPayload);

		expect(store.conflictPayload?.note_body).toBe('first change');

		// 同一 note_id の 2 件目 → 無視
		cap.emit(
			makePayload({
				note_id: '20260630120000',
				disk_body_hash: 'another_hash_888',
				note_body: 'second change'
			})
		);

		expect(store.conflictPayload?.note_body).toBe('first change');
		expect(store.state).toBe('compare');
	});

	it('spec.md#tp-wc-stop-unsubscribes — stop で subscribeFn が返した unsubscribe が 1 回呼ばれる', async () => {
		const cap = makeSubscribeCapture();
		const store = makeStore({ subscribeFn: cap.subscribeFn });

		await store.start();
		store.stop();

		expect(cap.unsubscribe).toHaveBeenCalledTimes(1);
	});

	it('spec.md#tp-wc-subscribe-failure-silent — subscribeFn が reject しても throw せず conflictPayload=null', async () => {
		const failingSubscribe: NonNullable<ConflictDialogStoreDeps['subscribeFn']> = async () => {
			throw new Error('subscribe failed');
		};
		const store = makeStore({ subscribeFn: failingSubscribe });

		await expect(store.start()).resolves.toBeUndefined();
		expect(store.conflictPayload).toBeNull();
		expect(store.state).toBe('hidden');
	});
});
