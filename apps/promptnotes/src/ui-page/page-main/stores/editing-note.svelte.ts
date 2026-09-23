export type EditingNoteState = {
	noteId: string | null;
	bodyHash: string | null;
};

export type EditingNoteStore = ReturnType<typeof createEditingNoteStore>;

export function createEditingNoteStore() {
	let state = $state<EditingNoteState>({ noteId: null, bodyHash: null });

	function setEditing(noteId: string | null, bodyHash: string | null): void {
		state = { noteId, bodyHash };
	}

	function clearIfCurrent(noteId: string): void {
		if (state.noteId === noteId) state = { noteId: null, bodyHash: null };
	}

	return {
		get noteId() {
			return state.noteId;
		},
		get bodyHash() {
			return state.bodyHash;
		},
		setEditing,
		clearIfCurrent
	};
}

export const editingNote = createEditingNoteStore();
