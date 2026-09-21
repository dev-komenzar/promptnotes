import type {
	ConflictDialogStoreDeps,
	NoteFileModifiedExternallyPayload
} from '../../../ui-widget/external-change-conflict/store.svelte';
import type {
	ExternalDeleteNoticeStoreDeps,
	NoteFileDeletedExternallyPayload
} from '../../../ui-widget/external-delete-notice/store.svelte';
import { feedStore, type FeedStore, type NoteSummary } from './feed.svelte';
import { editingNote, type EditingNoteStore } from './editing-note.svelte';
import { hashBody } from './body-hash';

export type ConflictHandler = (payload: NoteFileModifiedExternallyPayload) => void;

export type DeleteHandler = (payload: NoteFileDeletedExternallyPayload) => void;

export type ExternalChangeConflict = { noteId: string; localBody: string };

/** S20: the EDITING note disappeared from disk while being edited. */
export type ExternalDeletion = {
	noteId: string;
	localBody: string;
	tags: string[];
	createdAt: string;
	updatedAt: string;
};

export type ExternalChangeBridgeDeps = {
	feed?: Pick<FeedStore, 'notes'>;
	editing?: Pick<EditingNoteStore, 'noteId' | 'bodyHash' | 'setEditing'>;
	hashFn?: (body: string) => Promise<string>;
};

/**
 * Bridges the Tauri `notes-changed` signal into the screen-4 conflict dialog store.
 *
 * The Rust watcher only emits `notes-changed` (no payload), so the bridge re-reads the feed and
 * compares the edited note's disk body against the in-flight local body. When they differ while a
 * note is EDITING, it emits a `NoteFileModifiedExternally`-shaped payload to the dialog store and
 * reports the conflict so the caller can preserve the local body instead of clobbering it.
 */
export function createExternalChangeBridge(deps: ExternalChangeBridgeDeps = {}) {
	const feed = deps.feed ?? feedStore;
	const editing = deps.editing ?? editingNote;
	const hashFn = deps.hashFn ?? hashBody;
	let handler: ConflictHandler | null = null;
	let deleteHandler: DeleteHandler | null = null;

	function subscribe(next: ConflictHandler): () => void {
		handler = next;
		return () => {
			if (handler === next) handler = null;
		};
	}

	function subscribeDeletion(next: DeleteHandler): () => void {
		deleteHandler = next;
		return () => {
			if (deleteHandler === next) deleteHandler = null;
		};
	}

	async function notifyExternalModification(
		diskNotes: readonly NoteSummary[]
	): Promise<ExternalChangeConflict | null> {
		const noteId = editing.noteId;
		if (noteId === null) return null;
		const localNote = feed.notes.find((note) => note.id === noteId);
		const diskNote = diskNotes.find((note) => note.id === noteId);
		if (!localNote || !diskNote || diskNote.body === localNote.body) return null;

		editing.setEditing(noteId, await hashFn(localNote.body));
		handler?.({
			note_id: diskNote.id,
			disk_body_hash: await hashFn(diskNote.body),
			note_title: `${diskNote.id}.md`,
			note_body: diskNote.body,
			file_path: `${diskNote.id}.md`,
			detected_at: new Date().toISOString()
		});
		return { noteId, localBody: localNote.body };
	}

	async function notifyExternalDeletion(
		diskNotes: readonly NoteSummary[]
	): Promise<ExternalDeletion | null> {
		const noteId = editing.noteId;
		if (noteId === null) return null;
		const localNote = feed.notes.find((note) => note.id === noteId);
		if (!localNote) return null;
		if (diskNotes.some((note) => note.id === noteId)) return null;

		deleteHandler?.({
			note_id: localNote.id,
			note_title: `${localNote.id}.md`,
			note_body: localNote.body,
			file_path: `${localNote.id}.md`,
			detected_at: new Date().toISOString()
		});
		return {
			noteId,
			localBody: localNote.body,
			tags: [...localNote.tags],
			createdAt: localNote.created_at,
			updatedAt: localNote.updated_at
		};
	}

	function currentNoteId(): string | null {
		return editing.noteId;
	}

	function currentBodyHash(): string | null {
		return editing.bodyHash;
	}

	function currentLocalBody(): string {
		const noteId = editing.noteId;
		if (noteId === null) return '';
		return feed.notes.find((note) => note.id === noteId)?.body ?? '';
	}

	function conflictDeps(
		onApplyExternal: ConflictDialogStoreDeps['onApplyExternal']
	): ConflictDialogStoreDeps {
		return {
			subscribeFn: async (next) => subscribe(next),
			currentNoteId,
			currentBodyHash,
			onApplyExternal
		};
	}

	function deleteDeps(
		onSaveAsNew: ExternalDeleteNoticeStoreDeps['onSaveAsNew'],
		onDiscard: ExternalDeleteNoticeStoreDeps['onDiscard']
	): ExternalDeleteNoticeStoreDeps {
		return {
			subscribeFn: async (next) => subscribeDeletion(next),
			currentNoteId,
			onSaveAsNew,
			onDiscard
		};
	}

	return {
		subscribe,
		subscribeDeletion,
		notifyExternalModification,
		notifyExternalDeletion,
		currentNoteId,
		currentBodyHash,
		currentLocalBody,
		conflictDeps,
		deleteDeps
	};
}
