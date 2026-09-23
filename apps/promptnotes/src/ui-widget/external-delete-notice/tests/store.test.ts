import { describe, expect, it, vi } from 'vitest';
import {
	createExternalDeleteNoticeStore,
	type ExternalDeleteNoticeStoreDeps,
	type NoteFileDeletedExternallyPayload
} from '../store.svelte';

function makePayload(
	overrides: Partial<NoteFileDeletedExternallyPayload> = {}
): NoteFileDeletedExternallyPayload {
	return {
		note_id: '20260620120000',
		note_title: '20260620120000.md',
		note_body: 'hello local',
		file_path: '20260620120000.md',
		detected_at: '2026-09-21T00:00:00Z',
		...overrides
	};
}

type Capture = {
	emit: (payload: NoteFileDeletedExternallyPayload) => void;
	unsubscribe: ReturnType<typeof vi.fn>;
	subscribeFn: NonNullable<ExternalDeleteNoticeStoreDeps['subscribeFn']>;
};

function makeSubscribeCapture(): Capture {
	let stored: ((payload: NoteFileDeletedExternallyPayload) => void) | null = null;
	const unsubscribe = vi.fn();
	const subscribeFn = async (handler: (payload: NoteFileDeletedExternallyPayload) => void) => {
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

function makeStore(deps: Partial<ExternalDeleteNoticeStoreDeps> = {}) {
	return createExternalDeleteNoticeStore({
		currentNoteId: () => '20260620120000',
		onSaveAsNew: vi.fn(),
		onDiscard: vi.fn(),
		...deps
	});
}

describe('widget:widget-external-delete-notice store', () => {
	it('is hidden before any event is received', async () => {
		const cap = makeSubscribeCapture();
		const store = makeStore({ subscribeFn: cap.subscribeFn });

		await store.start();

		expect(store.payload).toBeNull();
		expect(store.state).toBe('hidden');
	});

	it('shows the notice for the deleted EDITING note', async () => {
		const cap = makeSubscribeCapture();
		const store = makeStore({ subscribeFn: cap.subscribeFn });

		await store.start();
		const payload = makePayload();
		cap.emit(payload);

		expect(store.payload).toStrictEqual(payload);
		expect(store.state).toBe('notice');
	});

	it('ignores events for a note that is not being edited', async () => {
		const cap = makeSubscribeCapture();
		const store = makeStore({ subscribeFn: cap.subscribeFn, currentNoteId: () => 'other' });

		await store.start();
		cap.emit(makePayload());

		expect(store.payload).toBeNull();
		expect(store.state).toBe('hidden');
	});

	it('ignores duplicate events while the notice is open', async () => {
		const cap = makeSubscribeCapture();
		const store = makeStore({ subscribeFn: cap.subscribeFn });

		await store.start();
		cap.emit(makePayload({ note_body: 'first' }));
		cap.emit(makePayload({ note_body: 'second' }));

		expect(store.payload?.note_body).toBe('first');
		expect(store.state).toBe('notice');
	});

	it('saveAsNew invokes onSaveAsNew once and hides', async () => {
		const cap = makeSubscribeCapture();
		const onSaveAsNew = vi.fn();
		const store = makeStore({ subscribeFn: cap.subscribeFn, onSaveAsNew });

		await store.start();
		const payload = makePayload();
		cap.emit(payload);

		await store.saveAsNew();

		expect(onSaveAsNew).toHaveBeenCalledExactlyOnceWith(payload);
		expect(store.payload).toBeNull();
		expect(store.state).toBe('hidden');
	});

	it('discard invokes onDiscard once and hides', async () => {
		const cap = makeSubscribeCapture();
		const onDiscard = vi.fn();
		const store = makeStore({ subscribeFn: cap.subscribeFn, onDiscard });

		await store.start();
		const payload = makePayload();
		cap.emit(payload);

		await store.discard();

		expect(onDiscard).toHaveBeenCalledExactlyOnceWith(payload);
		expect(store.payload).toBeNull();
		expect(store.state).toBe('hidden');
	});

	it('stop unsubscribes the listener exactly once', async () => {
		const cap = makeSubscribeCapture();
		const store = makeStore({ subscribeFn: cap.subscribeFn });

		await store.start();
		store.stop();

		expect(cap.unsubscribe).toHaveBeenCalledTimes(1);
	});

	it('is silent when subscribeFn rejects', async () => {
		const failingSubscribe: NonNullable<ExternalDeleteNoticeStoreDeps['subscribeFn']> = async () => {
			throw new Error('subscribe failed');
		};
		const store = makeStore({ subscribeFn: failingSubscribe });

		await expect(store.start()).resolves.toBeUndefined();
		expect(store.payload).toBeNull();
		expect(store.state).toBe('hidden');
	});
});
