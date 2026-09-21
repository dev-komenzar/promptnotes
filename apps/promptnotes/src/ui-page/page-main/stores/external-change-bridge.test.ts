import { describe, expect, it, vi } from 'vitest';
import type { NoteFileModifiedExternallyPayload } from '../../../ui-widget/external-change-conflict/store.svelte';
import { createExternalChangeBridge } from './external-change-bridge';
import type { NoteSummary } from './feed.svelte';

const NOTE_A = '20260620120000';

function makeNote(id: string, body: string): NoteSummary {
	return {
		id,
		body,
		tags: [],
		created_at: '2026-06-20T12:00:00Z',
		updated_at: '2026-06-20T12:00:00Z'
	};
}

function makeEditing(editing = false) {
	let state = {
		noteId: editing ? NOTE_A : null,
		bodyHash: editing ? 'h:hello local' : null
	};
	return {
		get noteId() {
			return state.noteId;
		},
		get bodyHash() {
			return state.bodyHash;
		},
		setEditing(noteId: string | null, bodyHash: string | null) {
			state = { noteId, bodyHash };
		}
	};
}

const hashFn = async (body: string) => `h:${body}`;

describe('page-main:external-change-bridge', () => {
	it('is silent when no note is EDITING', async () => {
		const emit = vi.fn();
		const bridge = createExternalChangeBridge({
			feed: { notes: [makeNote(NOTE_A, 'hello local')] },
			editing: makeEditing(false),
			hashFn
		});
		bridge.subscribe(emit);

		await expect(
			bridge.notifyExternalModification([makeNote(NOTE_A, 'hello world')])
		).resolves.toBeNull();
		expect(emit).not.toHaveBeenCalled();
	});

	it('is silent when the disk body equals the local body (no conflict)', async () => {
		const emit = vi.fn();
		const bridge = createExternalChangeBridge({
			feed: { notes: [makeNote(NOTE_A, 'hello local')] },
			editing: makeEditing(true),
			hashFn
		});
		bridge.subscribe(emit);

		await expect(
			bridge.notifyExternalModification([makeNote(NOTE_A, 'hello local')])
		).resolves.toBeNull();
		expect(emit).not.toHaveBeenCalled();
	});

	it('emits a NoteFileModifiedExternally payload when EDITING and the disk body differs', async () => {
		const emitted: NoteFileModifiedExternallyPayload[] = [];
		const editing = makeEditing(true);
		const bridge = createExternalChangeBridge({
			feed: { notes: [makeNote(NOTE_A, 'hello local')] },
			editing,
			hashFn
		});
		bridge.subscribe((payload) => emitted.push(payload));

		const conflict = await bridge.notifyExternalModification([makeNote(NOTE_A, 'hello world')]);

		expect(conflict).toStrictEqual({ noteId: NOTE_A, localBody: 'hello local' });
		expect(emitted).toHaveLength(1);
		expect(emitted[0]).toMatchObject({
			note_id: NOTE_A,
			disk_body_hash: 'h:hello world',
			note_title: `${NOTE_A}.md`,
			note_body: 'hello world'
		});
		expect(editing.bodyHash).toBe('h:hello local');
	});

	it('returns null when the edited note is missing from the disk snapshot', async () => {
		const emit = vi.fn();
		const bridge = createExternalChangeBridge({
			feed: { notes: [makeNote(NOTE_A, 'hello local')] },
			editing: makeEditing(true),
			hashFn
		});
		bridge.subscribe(emit);

		await expect(
			bridge.notifyExternalModification([makeNote('20260101000000', 'other')])
		).resolves.toBeNull();
		expect(emit).not.toHaveBeenCalled();
	});

	it('does not throw when no dialog handler is subscribed', async () => {
		const bridge = createExternalChangeBridge({
			feed: { notes: [makeNote(NOTE_A, 'hello local')] },
			editing: makeEditing(true),
			hashFn
		});

		await expect(
			bridge.notifyExternalModification([makeNote(NOTE_A, 'hello world')])
		).resolves.toStrictEqual({
			noteId: NOTE_A,
			localBody: 'hello local'
		});
	});

	it('currentLocalBody returns the in-flight body while EDITING and empty otherwise', () => {
		const editingBridge = createExternalChangeBridge({
			feed: { notes: [makeNote(NOTE_A, 'hello local')] },
			editing: makeEditing(true),
			hashFn
		});
		const idleBridge = createExternalChangeBridge({
			feed: { notes: [makeNote(NOTE_A, 'hello local')] },
			editing: makeEditing(false),
			hashFn
		});

		expect(editingBridge.currentLocalBody()).toBe('hello local');
		expect(idleBridge.currentLocalBody()).toBe('');
	});
});
